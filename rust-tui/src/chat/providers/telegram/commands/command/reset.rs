use super::super::*;

pub(super) async fn handle_reset_command(
    config: &Config,
    state: &mut TelegramState,
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
    let target_label = target.label.clone();
    let Some(pending) = remove_selected_target_pending_request(state) else {
        send_text(
            &config.telegram.bot_token,
            chat_id,
            &tg_fmt(locale, "reset.none", target_label),
        )
        .await?;
        return Ok(());
    };
    finalize_pending_feedback(config, &pending, tg(locale, "reset.status"));
    send_text(
        &config.telegram.bot_token,
        chat_id,
        &tg_fmt2(
            locale,
            "reset.done",
            pending.request_id,
            pending.target_label,
        ),
    )
    .await?;
    Ok(())
}
