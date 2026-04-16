use super::*;

#[derive(Clone, Debug)]
pub(super) struct PendingRolloutFailureResolution {
    pub(super) pending: PendingRequest,
    pub(super) failure: crate::chat::approval::CodexFailureEvent,
    pub(super) continuity: Option<crate::session_continuity::ContinuitySnapshot>,
}

pub(crate) async fn process_pending_rollout_failures(
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
        let reply = pending_failure_reply_text(
            locale,
            &resolution.pending,
            &resolution.failure,
            resolution.continuity.as_ref(),
        );
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

pub(super) fn pending_rollout_failure_check_due(pending: &PendingRequest, now: i64) -> bool {
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

pub(super) fn pending_failure_reply_text(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    failure: &crate::chat::approval::CodexFailureEvent,
    continuity: Option<&crate::session_continuity::ContinuitySnapshot>,
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
    if let Some(snapshot) = continuity {
        lines.extend(continuity_detail_lines(locale, snapshot));
    }

    format!(
        "{}\n\n{}:\n{}",
        lines.join("\n"),
        tg(locale, "failure.detail"),
        failure.message
    )
}

pub(super) fn detect_pending_rollout_failure_for_request(
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
    let continuity = crate::session_continuity::load_snapshot_for(
        snapshot.session_id.as_deref(),
        Some(&transcript_path),
    );
    let pending = remove_pending_request(state, request_id).unwrap_or(snapshot);
    Ok(Some(PendingRolloutFailureResolution {
        pending,
        failure,
        continuity,
    }))
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
