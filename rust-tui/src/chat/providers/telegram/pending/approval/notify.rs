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
