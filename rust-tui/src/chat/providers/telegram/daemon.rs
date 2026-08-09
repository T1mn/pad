mod auth {
    use super::super::*;

    #[derive(Default)]
    pub(super) struct TelegramAuthState {
        last_token: String,
        last_language: String,
    }

    pub(super) enum TelegramConfigReadiness {
        Ready,
        Retry,
        Exit,
    }

    pub(super) async fn prepare_config_for_polling(
        config: &mut Config,
        embedded: bool,
        auth_state: &mut TelegramAuthState,
    ) -> TelegramConfigReadiness {
        if !config.telegram.enabled {
            return retry_or_exit(embedded, "telegram: disabled in config, exiting").await;
        }
        if config.telegram.bot_token.trim().is_empty() {
            return retry_or_exit(embedded, "telegram: bot_token empty, exiting").await;
        }

        if needs_auth_refresh(config, auth_state) && !authenticate_bot(config, auth_state).await {
            sleep(Duration::from_secs(5)).await;
            return TelegramConfigReadiness::Retry;
        }

        TelegramConfigReadiness::Ready
    }

    async fn retry_or_exit(embedded: bool, exit_log: &str) -> TelegramConfigReadiness {
        if embedded {
            sleep(Duration::from_secs(1)).await;
            TelegramConfigReadiness::Retry
        } else {
            log_debug!("{}", exit_log);
            TelegramConfigReadiness::Exit
        }
    }

    fn needs_auth_refresh(config: &Config, auth_state: &TelegramAuthState) -> bool {
        config.telegram.bot_token != auth_state.last_token
            || config.telegram.bot_username.is_empty()
            || config.language != auth_state.last_language
    }

    async fn authenticate_bot(config: &mut Config, auth_state: &mut TelegramAuthState) -> bool {
        let auth_result = fetch_me(&config.telegram.bot_token).await;
        let me = match auth_result {
            Ok(me) => Some(me),
            Err(err) => {
                log_debug!("telegram: getMe failed: {}", err);
                None
            }
        };
        let Some(me) = me else {
            return false;
        };

        let username = me.username.unwrap_or_default();
        if config.telegram.bot_username != username {
            config.telegram.bot_username = username.clone();
            config.save_or_log();
        }
        if let Err(err) = set_my_commands(&config.telegram.bot_token, telegram_locale(config)).await
        {
            log_debug!("telegram: setMyCommands failed: {}", err);
        }
        auth_state.last_token = config.telegram.bot_token.clone();
        auth_state.last_language = config.language.clone();
        log_debug!("telegram: authenticated as @{}", username);
        true
    }
}
mod maintenance {
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
}
pub(crate) mod process;
mod run_loop {
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
}
pub(crate) mod state_io;
mod updates {
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
}

pub use process::{ensure_embedded_daemon_running, restart_daemon, sync_daemon};

pub async fn run_daemon() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_loop::run_daemon_loop(false).await
}
