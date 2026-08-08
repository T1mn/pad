mod candidates {
    use super::super::*;

    pub(super) fn approval_request_ids(state: &TelegramState) -> Vec<String> {
        state
            .pending_requests
            .iter()
            .filter(|pending| approval_scan_candidate(pending))
            .map(|pending| pending.request_id.clone())
            .collect()
    }

    pub(super) fn approval_snapshot(
        state: &TelegramState,
        request_id: &str,
    ) -> Option<PendingRequest> {
        let snapshot = state
            .pending_requests
            .iter()
            .find(|pending| pending.request_id == request_id)
            .cloned()?;
        approval_scan_candidate(&snapshot).then_some(snapshot)
    }

    fn approval_scan_candidate(pending: &PendingRequest) -> bool {
        pending.agent_kind == "codex"
            && (pending.accepted_at.is_some() || pending.phase == "awaiting_confirm")
    }
}
mod notify {
    use super::super::*;
    use super::scan::ApprovalScanOutcome;

    pub(super) async fn notify_approval_change(
        config: &Config,
        state: &mut TelegramState,
        request_id: &str,
        snapshot: &PendingRequest,
        outcome: ApprovalScanOutcome,
    ) -> TelegramResult<()> {
        refresh_pending_feedback(config, state, true);
        if let Some(request) = outcome.next_request {
            notify_new_approval_request(config, state, request_id, &request).await
        } else {
            log_cleared_approval(snapshot, outcome.previous_call_id);
            Ok(())
        }
    }

    async fn notify_new_approval_request(
        config: &Config,
        state: &TelegramState,
        request_id: &str,
        request: &CodexApprovalRequest,
    ) -> TelegramResult<()> {
        let Some(pending) = state
            .pending_requests
            .iter()
            .find(|pending| pending.request_id == request_id)
            .cloned()
        else {
            return Ok(());
        };
        send_codex_approval_prompt(config, &pending.chat_id, &pending, request).await?;
        play_sound_event(config, crate::sound::SoundEvent::Approval);
        log_debug!(
            "telegram: codex approval detected request={} pane={} call_id={}",
            pending.request_id,
            pending.pane_id,
            request.call_id
        );
        Ok(())
    }

    fn log_cleared_approval(snapshot: &PendingRequest, previous_call_id: Option<String>) {
        if let Some(previous_call_id) = previous_call_id {
            log_debug!(
                "telegram: codex approval cleared pane={} call_id={}",
                snapshot.pane_id,
                previous_call_id
            );
        }
    }
}
mod scan {
    use super::super::*;
    use std::path::Path;

    pub(super) struct ApprovalScanOutcome {
        pub(super) changed: bool,
        pub(super) previous_call_id: Option<String>,
        pub(super) next_request: Option<CodexApprovalRequest>,
    }

    pub(super) fn scan_and_apply_approval_state(
        state: &mut TelegramState,
        request_id: &str,
        snapshot: &PendingRequest,
        transcript_path: &str,
    ) -> TelegramResult<ApprovalScanOutcome> {
        let previous_call_id = snapshot.approval_call_id.clone();
        let scan_result = scan_codex_approval_updates(
            Path::new(transcript_path),
            snapshot.approval_scan_offset,
            current_approval_request(snapshot),
        )?;

        let next_request = scan_result.active_request.clone();
        let changed = previous_call_id.as_deref()
            != next_request
                .as_ref()
                .map(|request| request.call_id.as_str());

        apply_approval_scan_result(
            state,
            request_id,
            scan_result.next_offset,
            next_request.as_ref(),
        );

        Ok(ApprovalScanOutcome {
            changed,
            previous_call_id,
            next_request,
        })
    }

    fn current_approval_request(snapshot: &PendingRequest) -> Option<CodexApprovalRequest> {
        snapshot
            .approval_call_id
            .clone()
            .zip(snapshot.approval_justification.clone())
            .map(|(call_id, justification)| CodexApprovalRequest {
                call_id,
                justification,
            })
    }

    fn apply_approval_scan_result(
        state: &mut TelegramState,
        request_id: &str,
        next_offset: u64,
        next_request: Option<&CodexApprovalRequest>,
    ) {
        let Some(index) = pending_request_index_by_id(state, request_id) else {
            return;
        };
        let pending = &mut state.pending_requests[index];
        pending.approval_scan_offset = next_offset;
        match next_request {
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
}
mod transcript {
    use super::super::*;

    pub(super) fn ensure_approval_transcript_path(
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
            if pending.approval_scan_offset == 0 {
                pending.approval_scan_offset = transcript_len(&path).saturating_sub(32 * 1024);
            }
        }

        Ok(Some(path))
    }
}

use super::*;

pub(crate) async fn process_codex_pending_approval(
    config: &Config,
    state: &mut TelegramState,
) -> TelegramResult<()> {
    for request_id in candidates::approval_request_ids(state) {
        process_codex_pending_approval_for_request(config, state, &request_id).await?;
    }
    Ok(())
}

async fn process_codex_pending_approval_for_request(
    config: &Config,
    state: &mut TelegramState,
    request_id: &str,
) -> TelegramResult<()> {
    let Some(snapshot) = candidates::approval_snapshot(state, request_id) else {
        return Ok(());
    };

    let Some(transcript_path) =
        transcript::ensure_approval_transcript_path(state, request_id, &snapshot)?
    else {
        return Ok(());
    };

    let outcome =
        scan::scan_and_apply_approval_state(state, request_id, &snapshot, &transcript_path)?;
    if !outcome.changed {
        return Ok(());
    }

    notify::notify_approval_change(config, state, request_id, &snapshot, outcome).await
}
