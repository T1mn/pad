use super::chat_id::telegram_chat_id_value;
use super::client::telegram_api;
use super::{TELEGRAM_INTERACTIVE_TIMEOUT_SECS, TELEGRAM_STATUS_TIMEOUT_SECS};
use serde_json::json;

pub(in crate::chat::providers::telegram) async fn send_chat_action(
    token: &str,
    chat_id: &str,
    action: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _: serde_json::Value = telegram_api(
        token,
        "sendChatAction",
        &json!({
            "chat_id": telegram_chat_id_value(chat_id),
            "action": action,
        }),
        TELEGRAM_STATUS_TIMEOUT_SECS,
    )
    .await?;
    Ok(())
}

pub(in crate::chat::providers::telegram) async fn send_message_draft(
    token: &str,
    chat_id: &str,
    draft_id: i64,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _: serde_json::Value = telegram_api(
        token,
        "sendMessageDraft",
        &json!({
            "chat_id": telegram_chat_id_value(chat_id),
            "draft_id": draft_id,
            "text": text,
        }),
        TELEGRAM_STATUS_TIMEOUT_SECS,
    )
    .await?;
    Ok(())
}

pub(in crate::chat::providers::telegram) async fn answer_callback_query(
    token: &str,
    callback_query_id: &str,
    text: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let payload = match text {
        Some(text) => json!({
            "callback_query_id": callback_query_id,
            "text": text,
        }),
        None => json!({
            "callback_query_id": callback_query_id,
        }),
    };
    let _: serde_json::Value = telegram_api(
        token,
        "answerCallbackQuery",
        &payload,
        TELEGRAM_INTERACTIVE_TIMEOUT_SECS,
    )
    .await?;
    Ok(())
}
