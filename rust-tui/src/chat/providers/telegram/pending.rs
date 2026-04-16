use super::*;

pub(super) struct DraftFeedbackGate {
    pub(super) latest_seq: AtomicU64,
    pub(super) send_lock: AsyncMutex<()>,
}

#[derive(Clone, Debug)]
struct PendingRolloutFailureResolution {
    pending: PendingRequest,
    failure: crate::chat::approval::CodexFailureEvent,
}

pub(super) async fn process_pending_timeout(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let timed_out = state
        .pending_requests
        .iter()
        .filter(|pending| now_ts().saturating_sub(pending.sent_at) >= PENDING_TIMEOUT_SECS)
        .cloned()
        .collect::<Vec<_>>();

    for pending in timed_out {
        remove_pending_request(state, &pending.request_id);
        finalize_pending_feedback(config, &pending, tg(locale, "phase.completed"));
        send_text(
            &config.telegram.bot_token,
            &pending.chat_id,
            &tg_fmt(locale, "timeout", &pending.request_id),
        )
        .await?;
        play_sound_event(config, crate::sound::SoundEvent::Timeout);
    }

    Ok(())
}

pub(super) async fn process_pending_rollout_failures(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    let now = now_ts();
    let request_ids = state
        .pending_requests
        .iter()
        .filter(|pending| pending_rollout_failure_check_due(pending, now))
        .map(|pending| pending.request_id.clone())
        .collect::<Vec<_>>();

    for request_id in request_ids {
        let resolution = match detect_pending_rollout_failure_for_request(state, &request_id, now) {
            Ok(resolution) => resolution,
            Err(err) => {
                log_debug!(
                    "telegram: rollout failure detection failed request_id={} err={}",
                    request_id,
                    err
                );
                continue;
            }
        };
        let Some(resolution) = resolution else {
            continue;
        };

        let locale = telegram_locale(config);
        finalize_pending_feedback(config, &resolution.pending, tg(locale, "phase.failed"));
        let reply = pending_failure_reply_text(locale, &resolution.pending, &resolution.failure);
        if let Err(err) = send_text(
            &config.telegram.bot_token,
            &resolution.pending.chat_id,
            &reply,
        )
        .await
        {
            log_debug!(
                "telegram: rollout failure notification failed request_id={} err={}",
                resolution.pending.request_id,
                err
            );
        }
        play_sound_event(config, crate::sound::SoundEvent::Failure);
        log_debug!(
            "telegram: rollout failure released pending request={} pane={} error_info={} message={}",
            resolution.pending.request_id,
            resolution.pending.pane_id,
            resolution
                .failure
                .error_info
                .as_deref()
                .unwrap_or("unknown"),
            truncate_for_log(&resolution.failure.message, 240)
        );
    }

    Ok(())
}

pub(super) async fn process_pending_result_delivery(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    let request_ids = state
        .pending_requests
        .iter()
        .filter(|pending| {
            pending.phase == "delivering_result" && pending.delivery_retry_at <= now_ts()
        })
        .map(|pending| pending.request_id.clone())
        .collect::<Vec<_>>();

    for request_id in request_ids {
        if let Err(err) =
            deliver_pending_result(config, state, telegram_locale(config), &request_id).await
        {
            log_debug!(
                "telegram: result delivery retry failed request_id={} err={}",
                request_id,
                err
            );
        }
    }

    Ok(())
}

pub(super) async fn process_hook_journal(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    super::hooks::sync_state_from_disk_public(state);
    if state.pending_requests.is_empty() {
        state.journal_position = journal_len();
        return Ok(());
    }

    let path = crate::paths::hook_events_path();
    if !path.exists() {
        return Ok(());
    }

    let file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    if state.journal_position > len {
        state.journal_position = len;
    }
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(state.journal_position))?;

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        state.journal_position += line.len() as u64;
        super::hooks::sync_state_from_disk_public(state);
        if state.pending_requests.is_empty() {
            line.clear();
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }
        match serde_json::from_str::<HookEvent>(trimmed) {
            Ok(event) => {
                if !remember_processed_hook_event(state, &event) {
                    line.clear();
                    continue;
                }
                let _ = apply_hook_event_to_pending(config, state, &event).await?;
            }
            Err(err) => {
                log_debug!("telegram: invalid hook journal line: {}", err);
            }
        }
        line.clear();
    }

    Ok(())
}

pub(super) fn phase_label(locale: crate::i18n::Locale, phase: &str) -> String {
    match phase {
        "awaiting_submit" => tg(locale, "phase.awaiting_submit").to_string(),
        "awaiting_confirm" => tg(locale, "phase.awaiting_confirm").to_string(),
        "awaiting_stop" => tg(locale, "phase.accepted").to_string(),
        "delivering_result" => tg(locale, "phase.delivering").to_string(),
        _ => phase.to_string(),
    }
}

fn pending_metadata_lines(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    include_turn: bool,
) -> Vec<String> {
    let mut lines = vec![
        format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
        format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
    ];
    if let Some(session_id) = pending
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
    }
    if include_turn {
        if let Some(turn_id) = pending.turn_id.as_deref().filter(|value| !value.is_empty()) {
            lines.push(format!("{}: {}", tg(locale, "meta.turn"), turn_id));
        }
    }
    if !pending.working_dir.trim().is_empty() {
        lines.push(format!(
            "{}: {}",
            tg(locale, "meta.dir"),
            pending.working_dir
        ));
    }
    lines
}

pub(super) fn pending_status_summary_line(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
) -> String {
    format!(
        "{} • {} • {} • {}",
        pending.request_id,
        pending.pane_id,
        pending.target_label,
        phase_label(locale, &pending.phase)
    )
}

pub(super) fn pending_status_text(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    now: i64,
) -> String {
    if pending.approval_call_id.is_some() {
        let mut lines = vec![
            tg(locale, "phase.awaiting_confirm").to_string(),
            pending.target_label.clone(),
        ];
        lines.extend(pending_metadata_lines(locale, pending, false));
        if let Some(justification) = pending.approval_justification.as_deref() {
            lines.push(truncate_chars(justification, 220));
        }
        return lines.join("\n");
    }

    let headline = match pending.phase.as_str() {
        "awaiting_submit" => tg(locale, "phase.awaiting_submit").to_string(),
        "awaiting_stop" => match pending.accepted_at {
            Some(accepted_at) if now.saturating_sub(accepted_at) >= 4 => {
                tg_fmt(locale, "phase.working", now.saturating_sub(accepted_at))
            }
            _ => tg(locale, "phase.accepted").to_string(),
        },
        "delivering_result" => tg(locale, "phase.delivering").to_string(),
        _ => tg(locale, "phase.completed").to_string(),
    };

    let mut lines = vec![headline, pending.target_label.clone()];
    lines.extend(pending_metadata_lines(locale, pending, false));
    lines.join("\n")
}

pub(super) fn refresh_pending_feedback(config: &Config, state: &mut TelegramState, force: bool) {
    let locale = telegram_locale(config);
    let now = now_ts();

    for pending in &mut state.pending_requests {
        if !force {
            let Some(accepted_at) = pending.accepted_at else {
                continue;
            };
            if accepted_at <= 0 {
                continue;
            }
            if let Some(last_status_at) = pending.last_status_at {
                if now.saturating_sub(last_status_at) < 4 {
                    continue;
                }
            }
        }

        spawn_pending_feedback_update(
            config.telegram.bot_token.clone(),
            pending.chat_id.clone(),
            pending.draft_id,
            pending_status_text(locale, pending, now),
            true,
            tg(locale, "typing.action").to_string(),
        );
        pending.last_status_at = Some(now);
    }
}

pub(super) fn finalize_pending_feedback(config: &Config, pending: &PendingRequest, status: &str) {
    spawn_pending_feedback_update(
        config.telegram.bot_token.clone(),
        pending.chat_id.clone(),
        pending.draft_id,
        format!("{}\n{}", status, pending.target_label),
        false,
        String::new(),
    );
    let draft_id = pending.draft_id;
    tokio::spawn(async move {
        sleep(Duration::from_secs(5)).await;
        clear_draft_feedback_gate(draft_id);
    });
}

fn pending_rollout_failure_check_due(pending: &PendingRequest, now: i64) -> bool {
    if pending.agent_kind != "codex" {
        return false;
    }
    if !matches!(pending.phase.as_str(), "awaiting_stop" | "awaiting_confirm") {
        return false;
    }
    let Some(accepted_at) = pending.accepted_at else {
        return false;
    };
    if now.saturating_sub(accepted_at) < PENDING_FAILURE_SCAN_DELAY_SECS {
        return false;
    }
    pending
        .last_failure_check_at
        .map(|last_checked| now.saturating_sub(last_checked) >= PENDING_FAILURE_SCAN_INTERVAL_SECS)
        .unwrap_or(true)
}

fn pending_failure_reply_text(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    failure: &crate::chat::approval::CodexFailureEvent,
) -> String {
    let mut lines = vec![
        tg(locale, "failure.title").to_string(),
        format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
        format!("{}: {}", tg(locale, "meta.target"), pending.target_label),
        format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
    ];
    if let Some(session_id) = pending
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
    }
    if let Some(turn_id) = pending.turn_id.as_deref().filter(|value| !value.is_empty()) {
        lines.push(format!("{}: {}", tg(locale, "meta.turn"), turn_id));
    }
    if !pending.working_dir.trim().is_empty() {
        lines.push(format!(
            "{}: {}",
            tg(locale, "meta.dir"),
            pending.working_dir
        ));
    }
    if let Some(error_info) = failure
        .error_info
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "failure.kind"), error_info));
    }

    format!(
        "{}\n\n{}:\n{}",
        lines.join("\n"),
        tg(locale, "failure.detail"),
        failure.message
    )
}

fn detect_pending_rollout_failure_for_request(
    state: &mut TelegramState,
    request_id: &str,
    checked_at: i64,
) -> TelegramResult<Option<PendingRolloutFailureResolution>> {
    let Some(snapshot) = state
        .pending_requests
        .iter()
        .find(|pending| pending.request_id == request_id)
        .cloned()
    else {
        return Ok(None);
    };
    if !pending_rollout_failure_check_due(&snapshot, checked_at) {
        return Ok(None);
    }

    if let Some(index) = pending_request_index_by_id(state, request_id) {
        state.pending_requests[index].last_failure_check_at = Some(checked_at);
    }

    let Some(transcript_path) = ensure_pending_transcript_path(state, request_id, &snapshot)?
    else {
        return Ok(None);
    };
    let scan_result = crate::chat::approval::scan_codex_failure_updates(
        Path::new(&transcript_path),
        snapshot.failure_scan_offset,
        snapshot.turn_id.as_deref(),
    )?;

    if let Some(index) = pending_request_index_by_id(state, request_id) {
        let pending = &mut state.pending_requests[index];
        pending.failure_scan_offset = scan_result.next_offset;
    }

    let Some(failure) = scan_result.failure else {
        return Ok(None);
    };
    let pending = remove_pending_request(state, request_id).unwrap_or(snapshot);
    Ok(Some(PendingRolloutFailureResolution { pending, failure }))
}

fn ensure_pending_transcript_path(
    state: &mut TelegramState,
    request_id: &str,
    snapshot: &PendingRequest,
) -> TelegramResult<Option<String>> {
    if let Some(path) = snapshot.transcript_path.clone() {
        return Ok(Some(path));
    }

    let path = live_panels()
        .map_err(telegram_error)?
        .into_iter()
        .find(|panel| panel.pane_id == snapshot.pane_id)
        .and_then(|panel| panel.transcript_path);
    let Some(path) = path else {
        return Ok(None);
    };

    if let Some(index) = pending_request_index_by_id(state, request_id) {
        let pending = &mut state.pending_requests[index];
        pending.transcript_path = Some(path.clone());
        if pending.failure_scan_offset == 0 {
            pending.failure_scan_offset = if pending.result_scan_offset > 0 {
                pending.result_scan_offset
            } else {
                transcript_len(&path)
            };
        }
    }

    Ok(Some(path))
}

pub(super) fn completed_reply_text(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    result_text: &str,
) -> String {
    let mut lines = vec![
        tg(locale, "result.title").to_string(),
        format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
        format!("{}: {}", tg(locale, "meta.target"), pending.target_label),
        format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
    ];
    if let Some(session_id) = pending
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
    }
    if let Some(turn_id) = pending.turn_id.as_deref().filter(|value| !value.is_empty()) {
        lines.push(format!("{}: {}", tg(locale, "meta.turn"), turn_id));
    }
    if !pending.working_dir.trim().is_empty() {
        lines.push(format!(
            "{}: {}",
            tg(locale, "meta.dir"),
            pending.working_dir
        ));
    }
    format!("{}\n\n{}", lines.join("\n"), result_text)
}

pub(super) async fn deliver_pending_result(
    config: &Config,
    state: &mut TelegramState,
    locale: crate::i18n::Locale,
    request_id: &str,
) -> TelegramResult<()> {
    let Some(snapshot) = state
        .pending_requests
        .iter()
        .find(|pending| pending.request_id == request_id)
        .cloned()
    else {
        return Ok(());
    };
    if snapshot.phase != "delivering_result" {
        return Ok(());
    }

    let result_text = snapshot
        .completed_text
        .clone()
        .unwrap_or_else(|| tg(locale, "result.missing").to_string());
    let reply = completed_reply_text(locale, &snapshot, &result_text);
    match send_text(&config.telegram.bot_token, &snapshot.chat_id, &reply).await {
        Ok(()) => {
            finalize_pending_feedback(config, &snapshot, tg(locale, "phase.completed"));
            remove_pending_request(state, request_id);
            Ok(())
        }
        Err(err) => {
            if let Some(index) = pending_request_index_by_id(state, request_id) {
                let pending = &mut state.pending_requests[index];
                pending.delivery_attempts = pending.delivery_attempts.saturating_add(1);
                pending.delivery_retry_at = now_ts().saturating_add(RESULT_DELIVERY_RETRY_SECS);
                pending.last_status_at = None;
            }
            Err(err)
        }
    }
}

pub(super) async fn process_codex_pending_approval(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    let request_ids = state
        .pending_requests
        .iter()
        .filter(|pending| {
            pending.agent_kind == "codex"
                && (pending.accepted_at.is_some() || pending.phase == "awaiting_confirm")
        })
        .map(|pending| pending.request_id.clone())
        .collect::<Vec<_>>();

    for request_id in request_ids {
        process_codex_pending_approval_for_request(config, state, &request_id).await?;
    }

    Ok(())
}

async fn process_codex_pending_approval_for_request(
    config: &Config,
    state: &mut TelegramState,
    request_id: &str,
) -> TelegramResult<()> {
    let Some(snapshot) = state
        .pending_requests
        .iter()
        .find(|pending| pending.request_id == request_id)
        .cloned()
    else {
        return Ok(());
    };
    if snapshot.agent_kind != "codex" {
        return Ok(());
    }
    if snapshot.accepted_at.is_none() && snapshot.phase != "awaiting_confirm" {
        return Ok(());
    }

    let transcript_path = match snapshot.transcript_path.clone() {
        Some(path) => path,
        None => {
            let Some(path) = live_panels()
                .map_err(telegram_error)?
                .into_iter()
                .find(|panel| panel.pane_id == snapshot.pane_id)
                .and_then(|panel| panel.transcript_path)
            else {
                return Ok(());
            };
            if let Some(index) = pending_request_index_by_id(state, request_id) {
                let pending = &mut state.pending_requests[index];
                pending.transcript_path = Some(path.clone());
                if pending.approval_scan_offset == 0 {
                    pending.approval_scan_offset = transcript_len(&path).saturating_sub(32 * 1024);
                }
            }
            path
        }
    };

    let previous_call_id = snapshot.approval_call_id.clone();
    let current_request = snapshot
        .approval_call_id
        .clone()
        .zip(snapshot.approval_justification.clone())
        .map(|(call_id, justification)| CodexApprovalRequest {
            call_id,
            justification,
        });
    let scan_result = scan_codex_approval_updates(
        Path::new(&transcript_path),
        snapshot.approval_scan_offset,
        current_request,
    )?;

    let next_request = scan_result.active_request.clone();
    let changed = previous_call_id.as_deref()
        != next_request
            .as_ref()
            .map(|request| request.call_id.as_str());

    if let Some(index) = pending_request_index_by_id(state, request_id) {
        let pending = &mut state.pending_requests[index];
        pending.approval_scan_offset = scan_result.next_offset;
        match next_request.as_ref() {
            Some(request) => {
                pending.phase = "awaiting_confirm".to_string();
                pending.approval_call_id = Some(request.call_id.clone());
                pending.approval_justification = Some(request.justification.clone());
                pending.last_status_at = None;
            }
            None => {
                pending.approval_call_id = None;
                pending.approval_justification = None;
                if pending.phase == "awaiting_confirm" {
                    pending.phase = "awaiting_stop".to_string();
                }
                pending.last_status_at = None;
            }
        }
    }

    if !changed {
        return Ok(());
    }

    refresh_pending_feedback(config, state, true);
    if let Some(request) = next_request {
        let Some(pending) = state
            .pending_requests
            .iter()
            .find(|pending| pending.request_id == request_id)
            .cloned()
        else {
            return Ok(());
        };
        send_codex_approval_prompt(config, &pending.chat_id, &pending, &request).await?;
        play_sound_event(config, crate::sound::SoundEvent::Approval);
        log_debug!(
            "telegram: codex approval detected request={} pane={} call_id={}",
            pending.request_id,
            pending.pane_id,
            request.call_id
        );
    } else if let Some(previous_call_id) = previous_call_id {
        log_debug!(
            "telegram: codex approval cleared pane={} call_id={}",
            snapshot.pane_id,
            previous_call_id
        );
    }

    Ok(())
}

pub(super) fn pending_sent_ms(pending: &PendingRequest) -> i64 {
    if pending.sent_at_ms > 0 {
        pending.sent_at_ms
    } else {
        pending.sent_at.saturating_mul(1000)
    }
}

pub(super) fn pending_accepted_ms(pending: &PendingRequest) -> i64 {
    pending.accepted_at_ms.unwrap_or_else(|| {
        pending
            .accepted_at
            .unwrap_or(pending.sent_at)
            .saturating_mul(1000)
    })
}

pub(super) fn spawn_pending_feedback_update(
    token: String,
    chat_id: String,
    draft_id: i64,
    text: String,
    send_typing: bool,
    typing_action: String,
) {
    let gate = draft_feedback_gate(draft_id);
    let seq = gate.latest_seq.fetch_add(1, Ordering::SeqCst) + 1;
    tokio::spawn(async move {
        let _guard = gate.send_lock.lock().await;
        if gate.latest_seq.load(Ordering::SeqCst) != seq {
            return;
        }
        if send_typing {
            let _ = send_chat_action(&token, &chat_id, &typing_action).await;
        }
        let _ = send_message_draft(&token, &chat_id, draft_id, &text).await;
    });
}

pub(super) fn draft_feedback_gate(draft_id: i64) -> Arc<DraftFeedbackGate> {
    let mut gates = DRAFT_FEEDBACK_GATES
        .lock()
        .expect("draft feedback gates lock");
    gates
        .entry(draft_id)
        .or_insert_with(|| {
            Arc::new(DraftFeedbackGate {
                latest_seq: AtomicU64::new(0),
                send_lock: AsyncMutex::new(()),
            })
        })
        .clone()
}

pub(super) fn clear_draft_feedback_gate(draft_id: i64) {
    if let Ok(mut gates) = DRAFT_FEEDBACK_GATES.lock() {
        gates.remove(&draft_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_pending() -> PendingRequest {
        PendingRequest {
            request_id: "tg-1".into(),
            chat_id: "1".into(),
            pane_id: "%1".into(),
            agent_kind: "codex".into(),
            target_label: "CODEX • 1".into(),
            session_id: Some("session-1".into()),
            working_dir: "/tmp/test".into(),
            prompt_text: "hi".into(),
            prompt_hash: "abc".into(),
            turn_id: Some("turn-1".into()),
            sent_at: 100,
            sent_at_ms: 100_000,
            accepted_at: Some(105),
            accepted_at_ms: Some(105_000),
            last_status_at: None,
            draft_id: 7,
            phase: "awaiting_stop".into(),
            transcript_path: None,
            result_scan_offset: 0,
            failure_scan_offset: 0,
            last_failure_check_at: None,
            approval_scan_offset: 0,
            approval_call_id: None,
            approval_justification: None,
            completed_text: None,
            completed_source: None,
            delivery_attempts: 0,
            delivery_retry_at: 0,
        }
    }

    #[test]
    fn rollout_failure_check_waits_30_seconds_and_then_5_second_backoff() {
        let pending = sample_pending();
        assert!(!pending_rollout_failure_check_due(&pending, 134));
        assert!(pending_rollout_failure_check_due(&pending, 135));

        let mut checked = sample_pending();
        checked.last_failure_check_at = Some(135);
        assert!(!pending_rollout_failure_check_due(&checked, 139));
        assert!(pending_rollout_failure_check_due(&checked, 140));
    }

    #[test]
    fn detect_pending_rollout_failure_removes_pending_and_updates_scan_offset() {
        let path = std::env::temp_dir().join(format!(
            "pad-telegram-rollout-failure-{}.jsonl",
            std::process::id()
        ));
        let body = concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"still working\"}]}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"error\",\"message\":\"unexpected status 502 Bad Gateway\",\"codex_error_info\":\"other\"}}\n"
        );
        fs::write(&path, body).unwrap();

        let mut pending = sample_pending();
        pending.transcript_path = Some(path.to_string_lossy().into_owned());
        pending.failure_scan_offset = body.lines().next().unwrap().len() as u64 + 1;
        let mut state = TelegramState {
            pending_requests: vec![pending],
            ..TelegramState::default()
        };

        let resolution =
            detect_pending_rollout_failure_for_request(&mut state, "tg-1", 140).unwrap();
        let resolution = resolution.expect("failure resolution");
        assert_eq!(resolution.pending.request_id, "tg-1");
        assert_eq!(
            resolution.failure.message,
            "unexpected status 502 Bad Gateway"
        );
        assert_eq!(resolution.failure.error_info.as_deref(), Some("other"));
        assert!(state.pending_requests.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn detect_pending_rollout_failure_updates_last_check_when_no_error_is_found() {
        let path = std::env::temp_dir().join(format!(
            "pad-telegram-rollout-no-failure-{}.jsonl",
            std::process::id()
        ));
        let body = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"still working\"}]}}\n";
        fs::write(&path, body).unwrap();

        let mut pending = sample_pending();
        pending.transcript_path = Some(path.to_string_lossy().into_owned());
        let mut state = TelegramState {
            pending_requests: vec![pending],
            ..TelegramState::default()
        };

        let resolution =
            detect_pending_rollout_failure_for_request(&mut state, "tg-1", 140).unwrap();
        assert!(resolution.is_none());
        assert_eq!(state.pending_requests.len(), 1);
        assert_eq!(state.pending_requests[0].last_failure_check_at, Some(140));
        assert_eq!(
            state.pending_requests[0].failure_scan_offset,
            fs::metadata(&path).unwrap().len()
        );

        let _ = fs::remove_file(path);
    }
}
