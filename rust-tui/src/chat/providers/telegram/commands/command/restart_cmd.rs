use super::super::*;

pub(super) async fn handle_restart_command(config: &Config, chat_id: &str) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let plan = match restart::current_pad_restart_plan() {
        Ok(plan) => plan,
        Err(err_text) => {
            send_restart_failed(config, chat_id, locale, err_text).await?;
            return Ok(());
        }
    };
    let preparing_key = match plan.target {
        PadRestartTarget::RespawnPane(_) => "restart.preparing",
        PadRestartTarget::NewDetachedSession(_) => "restart.starting",
    };
    send_text(
        &config.telegram.bot_token,
        chat_id,
        tg(locale, preparing_key),
    )
    .await?;
    if let Err(err_text) = restart::execute_pad_restart_plan(&plan) {
        send_restart_failed(config, chat_id, locale, err_text).await?;
    }
    Ok(())
}

async fn send_restart_failed(
    config: &Config,
    chat_id: &str,
    locale: crate::i18n::Locale,
    err_text: String,
) -> TelegramResult<()> {
    send_text(
        &config.telegram.bot_token,
        chat_id,
        &tg_fmt(locale, "restart.failed", err_text),
    )
    .await?;
    play_sound_event(config, crate::sound::SoundEvent::Failure);
    Ok(())
}
