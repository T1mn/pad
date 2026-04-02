use super::*;

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
    let Some(pending_snapshot) = state.pending.as_ref().cloned() else {
        return Ok(());
    };
    if pending_snapshot.pane_id != pane_id {
        return Ok(());
    }
    if !remember_processed_hook_event(&mut state, event) {
        log_debug!(
            "telegram: skipped duplicate direct hook event={} pane={}",
            event.event,
            pane_id
        );
        return Ok(());
    }

    match event.event.as_str() {
        "user_prompt_submit" => {
            let matches_prompt = event
                .prompt
                .as_deref()
                .map(|prompt| {
                    format!("{:x}", md5::compute(prompt.as_bytes())) == pending_snapshot.prompt_hash
                })
                .unwrap_or(true);
            if matches_prompt {
                if let Some(pending) = state.pending.as_mut() {
                    pending.phase = "awaiting_stop".to_string();
                    pending.accepted_at = Some(now_ts());
                    if event.transcript_path.is_some() {
                        pending.transcript_path = event.transcript_path.clone();
                    }
                }
                refresh_pending_feedback(&config, &mut state, true);
                save_state(&state)?;
                log_debug!(
                    "telegram: direct hook advanced request {} to awaiting_stop dispatch_to_submit_ms={}",
                    pending_snapshot.request_id,
                    now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot))
                );
            }
        }
        "stop" => {
            let answer = event
                .last_assistant_message
                .clone()
                .filter(|text| !text.trim().is_empty())
                .or_else(|| latest_answer_for_pane(&pending_snapshot.pane_id));
            let request_id = pending_snapshot.request_id.clone();
            let chat_id = pending_snapshot.chat_id.clone();
            finalize_pending_feedback(&config, &pending_snapshot, tg(locale, "phase.completed"));
            state.pending = None;
            save_state(&state)?;
            let result_text = answer.unwrap_or_else(|| tg(locale, "result.missing").to_string());
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                &tg_fmt2(locale, "result.completed", &request_id, result_text),
            )
            .await
            .map_err(|err| io::Error::other(err.to_string()))?;
            log_debug!(
                "telegram: direct hook completed request {} total_ms={} run_ms={}",
                request_id,
                now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot)),
                now_ms_i64().saturating_sub(pending_accepted_ms(&pending_snapshot))
            );
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
    let Some(pending) = state.pending.as_ref() else {
        return false;
    };
    if state.last_journal_recovery_at == 0 {
        return true;
    }
    if !direct_hook_active {
        return now.saturating_sub(state.last_journal_recovery_at) >= 1;
    }
    if now.saturating_sub(state.last_journal_recovery_at) < JOURNAL_RECOVERY_RETRY_SECS {
        return false;
    }
    match pending.phase.as_str() {
        "awaiting_submit" => now.saturating_sub(pending.sent_at) >= JOURNAL_RECOVERY_STALL_SECS,
        "awaiting_stop" | "awaiting_confirm" => {
            now.saturating_sub(pending.accepted_at.unwrap_or(pending.sent_at))
                >= JOURNAL_RECOVERY_STALL_SECS
        }
        _ => false,
    }
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
        "{}|{}|{}|{}|{}",
        event.event,
        event.tmux.pane_id.as_deref().unwrap_or(""),
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
) -> Result<bool, Box<dyn std::error::Error>> {
    let locale = telegram_locale(config);
    let Some(pending_snapshot) = state.pending.as_ref().cloned() else {
        return Ok(true);
    };
    match event.event.as_str() {
        "user_prompt_submit" => {
            let matches_prompt = event
                .prompt
                .as_deref()
                .map(|prompt| {
                    format!("{:x}", md5::compute(prompt.as_bytes())) == pending_snapshot.prompt_hash
                })
                .unwrap_or(true);
            if matches_prompt {
                if let Some(pending) = state.pending.as_mut() {
                    pending.phase = "awaiting_stop".to_string();
                    pending.accepted_at = Some(now_ts());
                    pending.accepted_at_ms = Some(now_ms_i64());
                    if event.transcript_path.is_some() {
                        pending.transcript_path = event.transcript_path.clone();
                    }
                }
                refresh_pending_feedback(config, state, true);
                log_debug!(
                    "telegram: pending request {} reached awaiting_stop dispatch_to_submit_ms={}",
                    pending_snapshot.request_id,
                    now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot))
                );
            }
            Ok(false)
        }
        "stop" => {
            let answer = event
                .last_assistant_message
                .clone()
                .filter(|text| !text.trim().is_empty())
                .or_else(|| latest_answer_for_pane(&pending_snapshot.pane_id));
            let request_id = pending_snapshot.request_id.clone();
            let chat_id = pending_snapshot.chat_id.clone();
            finalize_pending_feedback(config, &pending_snapshot, tg(locale, "phase.completed"));
            state.pending = None;
            let result_text = answer.unwrap_or_else(|| tg(locale, "result.missing").to_string());
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                &tg_fmt2(locale, "result.completed", &request_id, result_text),
            )
            .await?;
            log_debug!(
                "telegram: completed request {} total_ms={} run_ms={}",
                request_id,
                now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot)),
                now_ms_i64().saturating_sub(pending_accepted_ms(&pending_snapshot))
            );
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub(super) fn sync_state_from_disk_public(state: &mut TelegramState) {
    sync_state_from_disk(state);
}
