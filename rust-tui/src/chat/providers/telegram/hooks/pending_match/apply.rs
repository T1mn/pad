use super::super::completion::{complete_pending_request, log_pending_completion};
use super::super::*;
use super::advance::advance_pending_to_awaiting_stop;
use super::matching::matching_pending_request_index;

pub(in crate::chat::providers::telegram) async fn apply_hook_event_to_pending(
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
