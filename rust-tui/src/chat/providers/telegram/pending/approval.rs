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
mod notify;
mod scan;
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
