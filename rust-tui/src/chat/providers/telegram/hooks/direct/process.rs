use super::super::super::{
    load_state, now_ms_i64, pending_request_index_by_pane, pending_sent_ms,
    refresh_pending_feedback, save_state, telegram_locale, Config,
};
use super::super::completion::{complete_pending_request, log_pending_completion};
use super::super::journal::remember_processed_hook_event;
use super::super::pending_match::{
    advance_pending_to_awaiting_stop, pending_can_complete_from_stop, pending_matches_submit_prompt,
};
use crate::hook::HookEvent;
use crate::log_debug;

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
    let Some(pending_index) = pending_request_index_by_pane(&state, pane_id) else {
        return Ok(());
    };
    let pending_snapshot = state.pending_requests[pending_index].clone();
    if !remember_processed_hook_event(&mut state, event) {
        log_debug!(
            "telegram: skipped duplicate direct hook event={} pane={}",
            event.event,
            pane_id
        );
        return Ok(());
    }

    match event.event.as_str() {
        "user_prompt_submit" if pending_matches_submit_prompt(&pending_snapshot, event) => {
            advance_pending_to_awaiting_stop(
                state.pending_requests.get_mut(pending_index),
                event,
                false,
            );
            refresh_pending_feedback(&config, &mut state, true);
            save_state(&state)?;
            log_debug!(
                "telegram: direct hook advanced request {} to awaiting_stop dispatch_to_submit_ms={}",
                pending_snapshot.request_id,
                now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot))
            );
        }
        "stop" => {
            if !pending_can_complete_from_stop(&pending_snapshot, event) {
                log_debug!(
                    "telegram: ignored stop for pane={} pending_phase={} pending_turn={:?} event_turn={:?}",
                    pane_id,
                    pending_snapshot.phase,
                    pending_snapshot.turn_id,
                    event.turn_id
                );
                return Ok(());
            }
            let completion = complete_pending_request(
                &config,
                &mut state,
                &pending_snapshot.request_id,
                &pending_snapshot,
                event,
                locale,
            )
            .await;
            save_state(&state)?;
            log_pending_completion("direct hook", &pending_snapshot, &completion);
        }
        _ => {}
    }

    Ok(())
}
