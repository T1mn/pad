use super::super::*;
use super::maintenance::save_state_quietly;
use super::state_io::serialized_state;

pub(super) async fn process_updates(
    config: &mut Config,
    state: &mut TelegramState,
    last_saved_state: &mut Option<String>,
) {
    let updates_result = get_updates(&config.telegram.bot_token, state.update_offset).await;
    let updates = match updates_result {
        Ok(updates) => Some(updates),
        Err(err) => {
            log_debug!("telegram: getUpdates failed: {}", err);
            None
        }
    };
    let Some(updates) = updates else {
        sleep(Duration::from_secs(2)).await;
        return;
    };

    for update in updates {
        reload_state_if_available(state, last_saved_state);
        if !mark_update_processed(state, update.update_id) {
            log_debug!(
                "telegram: skipping duplicate/stale update_id={} offset={}",
                update.update_id,
                state.update_offset
            );
            save_state_quietly(state, last_saved_state);
            continue;
        }
        save_state_quietly(state, last_saved_state);
        if let Err(err) = handle_update(config, state, update).await {
            log_debug!("telegram: update handling failed: {}", err);
        }
        save_state_quietly(state, last_saved_state);
    }
}

fn reload_state_if_available(state: &mut TelegramState, last_saved_state: &mut Option<String>) {
    if let Ok(latest_state) = load_state() {
        *last_saved_state = serialized_state(&latest_state).ok();
        *state = latest_state;
    }
}
