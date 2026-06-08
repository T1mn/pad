use super::super::*;

pub(super) async fn handle_stop_command(
    config: &Config,
    state: &TelegramState,
    chat_id: &str,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let Some(target) = state.selected_target.as_ref() else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            tg(locale, "target.none"),
        )
        .await?;
        return Ok(());
    };
    let stop_send_error = {
        let send_result = tmux_dispatch::send_escape(&target.pane_id);
        send_result.err().map(|err| err.to_string())
    };
    match stop_send_error {
        None => {
            send_text(
                &config.telegram.bot_token,
                chat_id,
                &tg_fmt(locale, "stop.sent", &target.label),
            )
            .await?;
        }
        Some(err_text) => {
            send_text(
                &config.telegram.bot_token,
                chat_id,
                &tg_fmt(locale, "stop.failed", err_text),
            )
            .await?;
            play_sound_event(config, crate::sound::SoundEvent::Failure);
        }
    }
    Ok(())
}
