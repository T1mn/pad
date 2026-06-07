use super::*;

mod completion;

#[cfg(test)]
use completion::resolve_pending_result_text;
use completion::{complete_pending_request, log_pending_completion};

pub(super) fn daemon_socket_is_active() -> bool {
    let path = crate::paths::telegram_hook_socket_path();
    path.exists() && StdUnixStream::connect(path).is_ok()
}

pub(super) fn start_direct_hook_listener() -> io::Result<()> {
    let socket_path = crate::paths::telegram_hook_socket_path();
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        if daemon_socket_is_active() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "telegram daemon socket already active at {}",
                    socket_path.display()
                ),
            ));
        }
        match std::fs::remove_file(&socket_path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    tokio::spawn(async move {
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                log_debug!("telegram: direct hook bind failed: {}", err);
                return;
            }
        };
        log_debug!(
            "telegram: direct hook listener on {}",
            socket_path.display()
        );

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        if let Err(err) = handle_direct_hook_stream(stream).await {
                            log_debug!("telegram: direct hook stream error: {}", err);
                        }
                    });
                }
                Err(err) => {
                    log_debug!("telegram: direct hook accept error: {}", err);
                    break;
                }
            }
        }
    });
    Ok(())
}

async fn handle_direct_hook_stream(
    stream: UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reader = TokioBufReader::new(stream);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let event: HookEvent = serde_json::from_str(&line)?;
        process_direct_hook_event(&event).await?;
    }

    Ok(())
}

pub(super) async fn process_direct_hook_event(
    event: &HookEvent,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load();
    let locale = telegram_locale(&config);
    if !config.telegram.enabled || config.telegram.bot_token.trim().is_empty() {
        return Ok(());
    }

    let Some(pane_id) = event.tmux.pane_id.as_deref() else {
        return Ok(());
    };

    let mut state = load_state().unwrap_or_default();
    let Some(pending_index) = pending_request_index_by_pane(&state, pane_id) else {
        return Ok(());
    };
    let pending_snapshot = state.pending_requests[pending_index].clone();
    if !remember_processed_hook_event(&mut state, event) {
        log_debug!(
            "telegram: skipped duplicate direct hook event={} pane={}",
            event.event,
            pane_id
        );
        return Ok(());
    }

    match event.event.as_str() {
        "user_prompt_submit" if pending_matches_submit_prompt(&pending_snapshot, event) => {
            advance_pending_to_awaiting_stop(
                state.pending_requests.get_mut(pending_index),
                event,
                false,
            );
            refresh_pending_feedback(&config, &mut state, true);
            save_state(&state)?;
            log_debug!(
                "telegram: direct hook advanced request {} to awaiting_stop dispatch_to_submit_ms={}",
                pending_snapshot.request_id,
                now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot))
            );
        }
        "stop" => {
            if !pending_can_complete_from_stop(&pending_snapshot, event) {
                log_debug!(
                    "telegram: ignored stop for pane={} pending_phase={} pending_turn={:?} event_turn={:?}",
                    pane_id,
                    pending_snapshot.phase,
                    pending_snapshot.turn_id,
                    event.turn_id
                );
                return Ok(());
            }
            let completion = complete_pending_request(
                &config,
                &mut state,
                &pending_snapshot.request_id,
                &pending_snapshot,
                event,
                locale,
            )
            .await;
            save_state(&state)?;
            log_pending_completion("direct hook", &pending_snapshot, &completion);
        }
        _ => {}
    }

    Ok(())
}

fn sync_state_from_disk(state: &mut TelegramState) {
    if let Ok(mut latest) = load_state() {
        latest.journal_position = latest.journal_position.max(state.journal_position);
        latest.last_journal_recovery_at = latest
            .last_journal_recovery_at
            .max(state.last_journal_recovery_at);
        *state = latest;
    }
}

pub(super) fn should_probe_hook_journal(state: &TelegramState) -> bool {
    should_probe_hook_journal_inner(state, daemon_socket_is_active(), now_ts())
}

pub(super) fn should_probe_hook_journal_inner(
    state: &TelegramState,
    direct_hook_active: bool,
    now: i64,
) -> bool {
    if state.pending_requests.is_empty() {
        return false;
    }
    if state.last_journal_recovery_at == 0 {
        return true;
    }
    if !direct_hook_active {
        return now.saturating_sub(state.last_journal_recovery_at) >= 1;
    }
    if now.saturating_sub(state.last_journal_recovery_at) < JOURNAL_RECOVERY_RETRY_SECS {
        return false;
    }
    state
        .pending_requests
        .iter()
        .any(|pending| match pending.phase.as_str() {
            "awaiting_submit" => now.saturating_sub(pending.sent_at) >= JOURNAL_RECOVERY_STALL_SECS,
            "awaiting_stop" | "awaiting_confirm" => {
                now.saturating_sub(pending.accepted_at.unwrap_or(pending.sent_at))
                    >= JOURNAL_RECOVERY_STALL_SECS
            }
            _ => false,
        })
}

pub(super) fn remember_processed_hook_event(state: &mut TelegramState, event: &HookEvent) -> bool {
    let signature = hook_event_signature(event);
    if recent_hook_signature_exists(&signature) {
        return false;
    }
    if state
        .processed_hook_signatures
        .iter()
        .any(|existing| existing == &signature)
    {
        return false;
    }
    state.processed_hook_signatures.push(signature);
    const MAX_PROCESSED_HOOKS: usize = 64;
    if state.processed_hook_signatures.len() > MAX_PROCESSED_HOOKS {
        let drop_count = state.processed_hook_signatures.len() - MAX_PROCESSED_HOOKS;
        state.processed_hook_signatures.drain(0..drop_count);
    }
    remember_recent_hook_signature(
        state
            .processed_hook_signatures
            .last()
            .expect("processed hook signature must exist"),
    );
    true
}

fn hook_event_signature(event: &HookEvent) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        event.event,
        event.tmux.pane_id.as_deref().unwrap_or(""),
        event.turn_id.as_deref().unwrap_or(""),
        event.timestamp.as_deref().unwrap_or(""),
        event.session_id.as_deref().unwrap_or(""),
        event
            .prompt
            .as_deref()
            .map(|prompt| format!("{:x}", md5::compute(prompt.as_bytes())))
            .unwrap_or_default()
    )
}

fn recent_hook_signature_exists(signature: &str) -> bool {
    RECENT_HOOK_SIGNATURES
        .lock()
        .map(|signatures| signatures.iter().any(|existing| existing == signature))
        .unwrap_or(false)
}

fn remember_recent_hook_signature(signature: &str) {
    if let Ok(mut signatures) = RECENT_HOOK_SIGNATURES.lock() {
        signatures.push(signature.to_string());
        const MAX_RECENT_HOOK_SIGNATURES: usize = 128;
        if signatures.len() > MAX_RECENT_HOOK_SIGNATURES {
            let drop_count = signatures.len() - MAX_RECENT_HOOK_SIGNATURES;
            signatures.drain(0..drop_count);
        }
    }
}

pub(super) async fn apply_hook_event_to_pending(
    config: &Config,
    state: &mut TelegramState,
    event: &HookEvent,
) -> TelegramResult<bool> {
    let locale = telegram_locale(config);
    let Some(pending_index) = matching_pending_request_index(state, event) else {
        return Ok(false);
    };
    let pending_snapshot = state.pending_requests[pending_index].clone();
    match event.event.as_str() {
        "user_prompt_submit" => {
            advance_pending_to_awaiting_stop(
                state.pending_requests.get_mut(pending_index),
                event,
                true,
            );
            refresh_pending_feedback(config, state, true);
            log_debug!(
                "telegram: pending request {} reached awaiting_stop dispatch_to_submit_ms={}",
                pending_snapshot.request_id,
                now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot))
            );
            Ok(false)
        }
        "stop" => {
            let completion = complete_pending_request(
                config,
                state,
                &pending_snapshot.request_id,
                &pending_snapshot,
                event,
                locale,
            )
            .await;
            log_pending_completion("journal", &pending_snapshot, &completion);
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub(super) fn sync_state_from_disk_public(state: &mut TelegramState) {
    sync_state_from_disk(state);
}

pub(super) fn matching_pending_request_index(
    state: &TelegramState,
    event: &HookEvent,
) -> Option<usize> {
    let pane_id = event.tmux.pane_id.as_deref()?;
    let pending_index = pending_request_index_by_pane(state, pane_id)?;
    let pending = state.pending_requests.get(pending_index)?;
    match event.event.as_str() {
        "user_prompt_submit" if pending_matches_submit_prompt(pending, event) => {
            Some(pending_index)
        }
        "stop" if pending_can_complete_from_stop(pending, event) => Some(pending_index),
        _ => None,
    }
}

fn pending_can_complete_from_stop(pending: &PendingRequest, event: &HookEvent) -> bool {
    matches!(pending.phase.as_str(), "awaiting_stop" | "awaiting_confirm")
        && hook_event_matches_pending_turn(pending, event)
}

fn hook_event_matches_pending_turn(pending: &PendingRequest, event: &HookEvent) -> bool {
    match (pending.turn_id.as_deref(), event.turn_id.as_deref()) {
        (Some(_), None) if pending.agent_kind == "codex" => false,
        (Some(expected), Some(actual)) => expected == actual,
        _ => true,
    }
}

fn pending_matches_submit_prompt(pending: &PendingRequest, event: &HookEvent) -> bool {
    event
        .prompt
        .as_deref()
        .map(|prompt| format!("{:x}", md5::compute(prompt.as_bytes())) == pending.prompt_hash)
        .unwrap_or(true)
}

fn advance_pending_to_awaiting_stop(
    pending: Option<&mut PendingRequest>,
    event: &HookEvent,
    record_accepted_at_ms: bool,
) {
    let Some(pending) = pending else {
        return;
    };
    pending.phase = "awaiting_stop".to_string();
    pending.accepted_at = Some(now_ts());
    if record_accepted_at_ms {
        pending.accepted_at_ms = Some(now_ms_i64());
    }
    if event.turn_id.is_some() {
        pending.turn_id = event.turn_id.clone();
    }
    if event.session_id.is_some() {
        pending.session_id = event.session_id.clone();
    }
    if event.cwd.is_some() {
        pending.working_dir = event.cwd.clone().unwrap_or_default();
    }
    if event.transcript_path.is_some() {
        pending.transcript_path = event.transcript_path.clone();
    }
    if pending.result_scan_offset == 0 {
        if let Some(path) = pending.transcript_path.as_deref() {
            pending.result_scan_offset = transcript_len(path);
        }
    }
    if pending.failure_scan_offset == 0 {
        if let Some(path) = pending.transcript_path.as_deref() {
            pending.failure_scan_offset = transcript_len(path);
        }
    }
}

#[cfg(test)]
mod tests;
