use super::super::*;
use super::auth::{prepare_config_for_polling, TelegramAuthState, TelegramConfigReadiness};
use super::maintenance::run_pending_maintenance;
use super::state_io::{save_state_if_changed, serialized_state};
use super::updates::process_updates;

pub(super) async fn run_daemon_loop(
    embedded: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mode = if embedded {
        "telegram-bot-embedded"
    } else {
        "telegram-bot"
    };
    let _status_guard =
        runtime_status::StatusGuard::new(crate::paths::telegram_bot_status_path(), mode)?;
    log_debug!(
        "telegram: daemon starting mode={}",
        if embedded { "embedded" } else { "standalone" }
    );

    let mut state = load_state().unwrap_or_default();
    let mut last_saved_state = serialized_state(&state).ok();
    if state.journal_position == 0 && state.pending_requests.is_empty() {
        state.journal_position = journal_len();
        save_state_if_changed(&state, &mut last_saved_state)?;
    }
    start_direct_hook_listener()?;

    let mut auth_state = TelegramAuthState::default();

    loop {
        let mut config = Config::load();
        if let Ok(latest_state) = load_state() {
            last_saved_state = serialized_state(&latest_state).ok();
            state = latest_state;
        }

        match prepare_config_for_polling(&mut config, embedded, &mut auth_state).await {
            TelegramConfigReadiness::Ready => {}
            TelegramConfigReadiness::Retry => continue,
            TelegramConfigReadiness::Exit => return Ok(()),
        }

        run_pending_maintenance(&config, &mut state, &mut last_saved_state).await;
        process_updates(&mut config, &mut state, &mut last_saved_state).await;
    }
}
