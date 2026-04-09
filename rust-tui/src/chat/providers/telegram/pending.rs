use super::*;

pub(super) struct DraftFeedbackGate {
    pub(super) latest_seq: AtomicU64,
    pub(super) send_lock: AsyncMutex<()>,
}

pub(super) async fn process_pending_timeout(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let Some(pending) = state.pending.as_ref() else {
        return Ok(());
    };
    if now_ts() - pending.sent_at < PENDING_TIMEOUT_SECS {
        return Ok(());
    }
    let pending_snapshot = pending.clone();
    let chat_id = pending.chat_id.clone();
    let request_id = pending.request_id.clone();
    finalize_pending_feedback(config, &pending_snapshot, tg(locale, "phase.completed"));
    state.pending = None;
    send_text(
        &config.telegram.bot_token,
        &chat_id,
        &tg_fmt(locale, "timeout", request_id),
    )
    .await?;
    Ok(())
}

pub(super) async fn process_pending_result_delivery(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    let Some(pending) = state.pending.as_ref() else {
        return Ok(());
    };
    if pending.phase != "delivering_result" {
        return Ok(());
    }
    if pending.delivery_retry_at > now_ts() {
        return Ok(());
    }
    if let Err(err) = deliver_pending_result(config, state, telegram_locale(config)).await {
        log_debug!("telegram: result delivery retry failed: {}", err);
    }
    Ok(())
}

pub(super) async fn process_hook_journal(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    super::hooks::sync_state_from_disk_public(state);
    let Some(_) = state.pending.clone() else {
        state.journal_position = journal_len();
        return Ok(());
    };

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
        let Some(current_pending) = state.pending.clone() else {
            line.clear();
            break;
        };
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
                if event.tmux.pane_id.as_deref() == Some(current_pending.pane_id.as_str())
                    && apply_hook_event_to_pending(config, state, &event).await?
                {
                    line.clear();
                    break;
                }
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
    format!("{}\n{}", headline, pending.target_label)
}

pub(super) fn refresh_pending_feedback(config: &Config, state: &mut TelegramState, force: bool) {
    let locale = telegram_locale(config);
    let now = now_ts();
    let Some(pending) = state.pending.as_mut() else {
        return;
    };

    if !force {
        let Some(accepted_at) = pending.accepted_at else {
            return;
        };
        if accepted_at <= 0 {
            return;
        }
        if let Some(last_status_at) = pending.last_status_at {
            if now.saturating_sub(last_status_at) < 4 {
                return;
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

pub(super) async fn deliver_pending_result(
    config: &Config,
    state: &mut TelegramState,
    locale: crate::i18n::Locale,
) -> TelegramResult<()> {
    let Some(snapshot) = state.pending.as_ref().cloned() else {
        return Ok(());
    };
    if snapshot.phase != "delivering_result" {
        return Ok(());
    }
    let result_text = snapshot
        .completed_text
        .clone()
        .unwrap_or_else(|| tg(locale, "result.missing").to_string());
    let reply = tg_fmt2(
        locale,
        "result.completed",
        &snapshot.request_id,
        result_text,
    );
    match send_text(&config.telegram.bot_token, &snapshot.chat_id, &reply).await {
        Ok(()) => {
            finalize_pending_feedback(config, &snapshot, tg(locale, "phase.completed"));
            state.pending = None;
            Ok(())
        }
        Err(err) => {
            if let Some(pending) = state.pending.as_mut() {
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
    let Some(snapshot) = state.pending.as_ref().cloned() else {
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
            if let Some(pending) = state.pending.as_mut() {
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

    if let Some(pending) = state.pending.as_mut() {
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
        let pending = state.pending.as_ref().expect("pending must exist");
        send_codex_approval_prompt(config, &pending.chat_id, pending, &request).await?;
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
