use super::*;

pub(crate) async fn handle_update(
    config: &mut Config,
    state: &mut TelegramState,
    update: TelegramUpdate,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    if let Some(callback_query) = update.callback_query {
        return handle_callback_query(config, state, callback_query).await;
    }

    let Some(message) = update.message else {
        return Ok(());
    };

    if message.chat.kind != "private" {
        log_debug!("telegram: ignoring non-private chat {}", message.chat.id);
        return Ok(());
    }

    let chat_id = message.chat.id.to_string();
    let text = message.text.unwrap_or_default();
    log_debug!(
        "telegram: incoming message chat={} msg_id={} text={}",
        chat_id,
        message.message_id,
        truncate_for_log(&text, 200)
    );

    if config.telegram.chat_id.is_empty() {
        if text.starts_with("/start") {
            config.telegram.chat_id = chat_id.clone();
            config.save();
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                tg(locale, "bind.success"),
            )
            .await?;
        } else {
            send_text(
                &config.telegram.bot_token,
                &chat_id,
                tg(locale, "bind.start_required"),
            )
            .await?;
        }
        return Ok(());
    }

    if config.telegram.chat_id != chat_id {
        send_text(
            &config.telegram.bot_token,
            &chat_id,
            tg(locale, "bind.other_chat"),
        )
        .await?;
        return Ok(());
    }

    if text.starts_with('/') {
        super::handle_command(config, state, &chat_id, &text).await?;
    } else {
        super::handle_plain_text(config, state, &chat_id, &text).await?;
    }

    Ok(())
}
