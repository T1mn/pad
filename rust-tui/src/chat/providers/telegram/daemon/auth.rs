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
        config.save();
    }
    if let Err(err) = set_my_commands(&config.telegram.bot_token, telegram_locale(config)).await {
        log_debug!("telegram: setMyCommands failed: {}", err);
    }
    auth_state.last_token = config.telegram.bot_token.clone();
    auth_state.last_language = config.language.clone();
    log_debug!("telegram: authenticated as @{}", username);
    true
}
