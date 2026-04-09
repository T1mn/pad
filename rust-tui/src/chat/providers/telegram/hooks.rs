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
            if pending_matches_submit_prompt(&pending_snapshot, event) {
                advance_pending_to_awaiting_stop(state.pending.as_mut(), event, false);
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
            if !hook_event_matches_pending_turn(&pending_snapshot, event) {
                log_debug!(
                    "telegram: ignored stop for pane={} pending_turn={:?} event_turn={:?}",
                    pane_id,
                    pending_snapshot.turn_id,
                    event.turn_id
                );
                return Ok(());
            }
            let completion =
                complete_pending_request(&config, &mut state, &pending_snapshot, event, locale)
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
    let Some(pending_snapshot) = state.pending.as_ref().cloned() else {
        return Ok(true);
    };
    match event.event.as_str() {
        "user_prompt_submit" => {
            if pending_matches_submit_prompt(&pending_snapshot, event) {
                advance_pending_to_awaiting_stop(state.pending.as_mut(), event, true);
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
            if !hook_event_matches_pending_turn(&pending_snapshot, event) {
                log_debug!(
                    "telegram: ignored journal stop for pane={} pending_turn={:?} event_turn={:?}",
                    pending_snapshot.pane_id,
                    pending_snapshot.turn_id,
                    event.turn_id
                );
                return Ok(false);
            }
            let completion =
                complete_pending_request(config, state, &pending_snapshot, event, locale).await;
            log_pending_completion("journal", &pending_snapshot, &completion);
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub(super) fn sync_state_from_disk_public(state: &mut TelegramState) {
    sync_state_from_disk(state);
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
    if event.transcript_path.is_some() {
        pending.transcript_path = event.transcript_path.clone();
    }
    if pending.result_scan_offset == 0 {
        if let Some(path) = pending.transcript_path.as_deref() {
            pending.result_scan_offset = transcript_len(path);
        }
    }
}

async fn complete_pending_request(
    config: &Config,
    state: &mut TelegramState,
    pending_snapshot: &PendingRequest,
    event: &HookEvent,
    locale: crate::i18n::Locale,
) -> PendingCompletionOutcome {
    let resolved = await_pending_result_text(pending_snapshot, event).await;
    cache_pending_completion(state.pending.as_mut(), locale, &resolved);
    match deliver_pending_result(config, state, locale).await {
        Ok(()) => PendingCompletionOutcome::delivered(&resolved),
        Err(err) => PendingCompletionOutcome::deferred(&resolved, err.to_string()),
    }
}

fn cache_pending_completion(
    pending: Option<&mut PendingRequest>,
    locale: crate::i18n::Locale,
    resolved: &ResolvedPendingResult,
) {
    let Some(pending) = pending else {
        return;
    };
    pending.phase = "delivering_result".to_string();
    pending.completed_text = Some(
        resolved
            .text
            .clone()
            .unwrap_or_else(|| tg(locale, "result.missing").to_string()),
    );
    pending.completed_source = Some(resolved.source.to_string());
    pending.delivery_retry_at = 0;
    pending.last_status_at = None;
}

fn log_pending_completion(
    channel: &str,
    pending_snapshot: &PendingRequest,
    completion: &PendingCompletionOutcome,
) {
    if let Some(err) = completion.error.as_deref() {
        log_debug!(
            "telegram: {} deferred result delivery request {} total_ms={} run_ms={} result_source={} result_chars={} err={}",
            channel,
            pending_snapshot.request_id,
            now_ms_i64().saturating_sub(pending_sent_ms(pending_snapshot)),
            now_ms_i64().saturating_sub(pending_accepted_ms(pending_snapshot)),
            completion.source,
            completion.char_count,
            err
        );
    } else {
        log_debug!(
            "telegram: {} completed request {} total_ms={} run_ms={} result_source={} result_chars={}",
            channel,
            pending_snapshot.request_id,
            now_ms_i64().saturating_sub(pending_sent_ms(pending_snapshot)),
            now_ms_i64().saturating_sub(pending_accepted_ms(pending_snapshot)),
            completion.source,
            completion.char_count
        );
    }
}

async fn await_pending_result_text(
    pending: &PendingRequest,
    event: &HookEvent,
) -> ResolvedPendingResult {
    let initial = resolve_pending_result_text(pending, event);
    if pending.agent_kind != "codex" || initial.source == "transcript_completion" {
        return initial;
    }

    // Codex can emit Stop before the final assistant message is appended to the
    // transcript. Give the transcript a short window to catch up before falling
    // back to the hook payload.
    const RETRIES: usize = 24;
    const SLEEP_MS: u64 = 250;
    for attempt in 1..=RETRIES {
        sleep(Duration::from_millis(SLEEP_MS)).await;
        let retried = resolve_pending_result_text(pending, event);
        if retried.source == "transcript_completion" {
            log_debug!(
                "telegram: codex transcript caught up pane={} after_retry={} wait_ms={}",
                pending.pane_id,
                attempt,
                attempt as u64 * SLEEP_MS
            );
            return retried;
        }
    }

    log_debug!(
        "telegram: codex transcript still missing pane={} after_wait_ms={} fallback_source={}",
        pending.pane_id,
        RETRIES as u64 * SLEEP_MS,
        initial.source
    );
    initial
}

fn resolve_pending_result_text(
    pending: &PendingRequest,
    event: &HookEvent,
) -> ResolvedPendingResult {
    let hook_text = event
        .last_assistant_message
        .clone()
        .filter(|text| !text.trim().is_empty());
    let transcript_text = pending.transcript_path.as_deref().and_then(|path| {
        crate::chat::approval::scan_codex_answer_updates(
            std::path::Path::new(path),
            pending.result_scan_offset,
            pending.turn_id.as_deref().or(event.turn_id.as_deref()),
        )
        .ok()
        .flatten()
    });

    if pending.agent_kind == "codex" {
        if let (Some(hook), Some(transcript)) = (hook_text.as_deref(), transcript_text.as_deref()) {
            if hook.trim() != transcript.trim() {
                log_debug!(
                    "telegram: codex stop payload mismatch pane={} hook_chars={} transcript_chars={} preferring=transcript_completion",
                    pending.pane_id,
                    hook.chars().count(),
                    transcript.chars().count()
                );
            }
        }
        if transcript_text.is_some() {
            return ResolvedPendingResult::new(transcript_text, "transcript_completion");
        }
        if hook_text.is_some() {
            return ResolvedPendingResult::new(hook_text, "hook_payload");
        }
        return ResolvedPendingResult::new(None, "missing");
    }

    if hook_text.is_some() {
        ResolvedPendingResult::new(hook_text, "hook_payload")
    } else if transcript_text.is_some() {
        ResolvedPendingResult::new(transcript_text, "transcript_delta")
    } else {
        ResolvedPendingResult::new(None, "missing")
    }
}

struct ResolvedPendingResult {
    text: Option<String>,
    source: &'static str,
    char_count: usize,
}

impl ResolvedPendingResult {
    fn new(text: Option<String>, source: &'static str) -> Self {
        let char_count = text
            .as_ref()
            .map(|value| value.chars().count())
            .unwrap_or(0);
        Self {
            text,
            source,
            char_count,
        }
    }
}

struct PendingCompletionOutcome {
    source: &'static str,
    char_count: usize,
    error: Option<String>,
}

impl PendingCompletionOutcome {
    fn delivered(resolved: &ResolvedPendingResult) -> Self {
        Self {
            source: resolved.source,
            char_count: resolved.char_count,
            error: None,
        }
    }

    fn deferred(resolved: &ResolvedPendingResult, error: String) -> Self {
        Self {
            source: resolved.source,
            char_count: resolved.char_count,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::HookTmuxInfo;
    use std::fs;

    #[test]
    fn codex_stop_prefers_transcript_completion_over_stale_hook_payload() {
        let path = std::env::temp_dir().join(format!(
            "pad-codex-stop-prefer-transcript-{}.jsonl",
            std::process::id()
        ));
        let old = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"old answer\"}]}}\n";
        let new = concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"final_answer\",\"content\":[{\"type\":\"output_text\",\"text\":\"new answer\"}]}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\",\"turn_id\":\"turn-new\",\"last_agent_message\":\"new answer\"}}\n"
        );
        fs::write(&path, format!("{old}{new}")).unwrap();

        let pending = PendingRequest {
            request_id: "tg-1".into(),
            chat_id: "1".into(),
            pane_id: "%1".into(),
            agent_kind: "codex".into(),
            target_label: "CODEX • test".into(),
            prompt_text: "hi".into(),
            prompt_hash: "abc".into(),
            turn_id: Some("turn-new".into()),
            sent_at: 100,
            sent_at_ms: 100_000,
            accepted_at: Some(101),
            accepted_at_ms: Some(101_000),
            last_status_at: None,
            draft_id: 1,
            phase: "awaiting_stop".into(),
            transcript_path: Some(path.to_string_lossy().into_owned()),
            result_scan_offset: old.len() as u64,
            approval_scan_offset: 0,
            approval_call_id: None,
            approval_justification: None,
            completed_text: None,
            completed_source: None,
            delivery_attempts: 0,
            delivery_retry_at: 0,
        };
        let event = HookEvent {
            event: "stop".into(),
            turn_id: Some("turn-old".into()),
            session_id: Some("s1".into()),
            transcript_path: pending.transcript_path.clone(),
            cwd: None,
            prompt: None,
            last_assistant_message: Some("stale hook payload".into()),
            timestamp: Some("2026-04-07T00:00:00Z".into()),
            tmux: HookTmuxInfo {
                pane_id: Some("%1".into()),
                session_name: Some("0".into()),
                window_index: Some("1".into()),
                pane_index: Some("1".into()),
                pane_current_path: None,
            },
        };

        let resolved = resolve_pending_result_text(&pending, &event);
        assert_eq!(resolved.source, "transcript_completion");
        assert_eq!(resolved.text.as_deref(), Some("new answer"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn pending_turn_must_match_stop_turn_when_both_exist() {
        let pending = PendingRequest {
            request_id: "tg-1".into(),
            chat_id: "1".into(),
            pane_id: "%1".into(),
            agent_kind: "codex".into(),
            target_label: "CODEX • test".into(),
            prompt_text: "hi".into(),
            prompt_hash: "abc".into(),
            turn_id: Some("turn-a".into()),
            sent_at: 100,
            sent_at_ms: 100_000,
            accepted_at: Some(101),
            accepted_at_ms: Some(101_000),
            last_status_at: None,
            draft_id: 1,
            phase: "awaiting_stop".into(),
            transcript_path: None,
            result_scan_offset: 0,
            approval_scan_offset: 0,
            approval_call_id: None,
            approval_justification: None,
            completed_text: None,
            completed_source: None,
            delivery_attempts: 0,
            delivery_retry_at: 0,
        };
        let event = HookEvent {
            event: "stop".into(),
            turn_id: Some("turn-b".into()),
            session_id: Some("s1".into()),
            transcript_path: None,
            cwd: None,
            prompt: None,
            last_assistant_message: Some("wrong turn".into()),
            timestamp: Some("2026-04-07T00:00:00Z".into()),
            tmux: HookTmuxInfo {
                pane_id: Some("%1".into()),
                session_name: Some("0".into()),
                window_index: Some("1".into()),
                pane_index: Some("1".into()),
                pane_current_path: None,
            },
        };

        assert!(!hook_event_matches_pending_turn(&pending, &event));
    }

    #[test]
    fn codex_stop_without_turn_id_is_ignored_when_pending_turn_exists() {
        let pending = PendingRequest {
            request_id: "tg-1".into(),
            chat_id: "1".into(),
            pane_id: "%1".into(),
            agent_kind: "codex".into(),
            target_label: "CODEX • test".into(),
            prompt_text: "hi".into(),
            prompt_hash: "abc".into(),
            turn_id: Some("turn-a".into()),
            sent_at: 100,
            sent_at_ms: 100_000,
            accepted_at: Some(101),
            accepted_at_ms: Some(101_000),
            last_status_at: None,
            draft_id: 1,
            phase: "awaiting_stop".into(),
            transcript_path: None,
            result_scan_offset: 0,
            approval_scan_offset: 0,
            approval_call_id: None,
            approval_justification: None,
            completed_text: None,
            completed_source: None,
            delivery_attempts: 0,
            delivery_retry_at: 0,
        };
        let event = HookEvent {
            event: "stop".into(),
            turn_id: None,
            session_id: Some("s1".into()),
            transcript_path: None,
            cwd: None,
            prompt: None,
            last_assistant_message: Some("missing turn".into()),
            timestamp: Some("2026-04-08T00:00:00Z".into()),
            tmux: HookTmuxInfo {
                pane_id: Some("%1".into()),
                session_name: Some("0".into()),
                window_index: Some("1".into()),
                pane_index: Some("1".into()),
                pane_current_path: None,
            },
        };

        assert!(!hook_event_matches_pending_turn(&pending, &event));
    }
}
