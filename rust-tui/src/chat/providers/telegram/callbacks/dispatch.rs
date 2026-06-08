mod approval_handler;
mod use_pane;

use super::super::*;
use approval_handler::handle_approval_callback;
use use_pane::handle_use_pane_callback;

pub(in crate::chat::providers::telegram) async fn handle_callback_query(
    config: &Config,
    state: &mut TelegramState,
    query: TelegramCallbackQuery,
) -> TelegramResult<()> {
    let locale = telegram_locale(config);
    let Some(message) = query.message else {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.invalid")),
        )
        .await?;
        return Ok(());
    };
    if message.chat.kind != "private" {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.private_only")),
        )
        .await?;
        return Ok(());
    }

    let chat_id = message.chat.id.to_string();
    if !config.telegram.chat_id.is_empty() && config.telegram.chat_id != chat_id {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.bound_other")),
        )
        .await?;
        return Ok(());
    }

    let Some(data) = query.data.as_deref() else {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.no_data")),
        )
        .await?;
        return Ok(());
    };

    if let Some(page) = HelpPage::from_callback(data) {
        edit_help_message(config, state, &chat_id, message.message_id, page).await?;
        answer_callback_query(&config.telegram.bot_token, &query.id, None).await?;
    } else if data == "help:list" {
        send_agent_list(config, state, &chat_id).await?;
        answer_callback_query(&config.telegram.bot_token, &query.id, None).await?;
    } else if data == "help:padstatus" {
        send_pad_status_report(config, state, &chat_id).await?;
        answer_callback_query(&config.telegram.bot_token, &query.id, None).await?;
    } else if let Some(pane_id) = data.strip_prefix("use-pane:") {
        handle_use_pane_callback(config, state, &query.id, &chat_id, pane_id, locale).await?;
    } else if data.starts_with("approval:") {
        handle_approval_callback(config, state, &query.id, data, locale).await?;
    } else {
        answer_callback_query(
            &config.telegram.bot_token,
            &query.id,
            Some(tg(locale, "callback.unknown")),
        )
        .await?;
    }

    Ok(())
}
