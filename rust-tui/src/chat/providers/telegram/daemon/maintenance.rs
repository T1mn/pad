use super::super::*;
use super::state_io::save_state_if_changed;

pub(super) async fn run_pending_maintenance(
    config: &Config,
    state: &mut TelegramState,
    last_saved_state: &mut Option<String>,
) {
    if let Err(err) = process_pending_timeout(config, state).await {
        log_debug!("telegram: pending timeout handling failed: {}", err);
    }
    save_state_quietly(state, last_saved_state);

    if let Err(err) = process_pending_result_delivery(config, state).await {
        log_debug!("telegram: pending result delivery failed: {}", err);
    }
    save_state_quietly(state, last_saved_state);

    if should_probe_hook_journal(state) {
        state.last_journal_recovery_at = now_ts();
        if let Err(err) = process_hook_journal(config, state).await {
            log_debug!("telegram: hook journal processing failed: {}", err);
        }
        save_state_quietly(state, last_saved_state);
    }

    if let Err(err) = process_pending_rollout_failures(config, state).await {
        log_debug!("telegram: pending rollout failure handling failed: {}", err);
    }
    save_state_quietly(state, last_saved_state);

    if let Err(err) = process_codex_pending_approval(config, state).await {
        log_debug!("telegram: codex approval processing failed: {}", err);
    }
    save_state_quietly(state, last_saved_state);

    refresh_pending_feedback(config, state, false);
    save_state_quietly(state, last_saved_state);
}

pub(super) fn save_state_quietly(state: &TelegramState, last_saved_state: &mut Option<String>) {
    let _ = save_state_if_changed(state, last_saved_state);
}
