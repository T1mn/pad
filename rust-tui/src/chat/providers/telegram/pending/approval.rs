use super::*;

pub(crate) async fn process_codex_pending_approval(
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
