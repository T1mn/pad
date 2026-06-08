use super::client::telegram_api;
use super::text::chunk_text;
use super::{TELEGRAM_INTERACTIVE_TIMEOUT_SECS, TELEGRAM_MAX_TEXT_LEN, TELEGRAM_TIMEOUT_SECS};
use serde_json::json;

pub(in crate::chat::providers::telegram) async fn send_text(
    token: &str,
    chat_id: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let chunks = chunk_text(text, TELEGRAM_MAX_TEXT_LEN);
    let total = chunks.len();
    for (idx, chunk) in chunks.into_iter().enumerate() {
        let body = if total > 1 {
            format!("({}/{})\n{}", idx + 1, total, chunk)
        } else {
            chunk
        };
        let _: serde_json::Value = telegram_api(
            token,
            "sendMessage",
            &json!({
                "chat_id": chat_id,
                "text": body,
            }),
            TELEGRAM_TIMEOUT_SECS,
        )
        .await?;
    }
    Ok(())
}

pub(in crate::chat::providers::telegram) async fn send_message(
    token: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    telegram_api(token, "sendMessage", payload, TELEGRAM_TIMEOUT_SECS).await
}

pub(in crate::chat::providers::telegram) async fn edit_message(
    token: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    telegram_api(
        token,
        "editMessageText",
        payload,
        TELEGRAM_INTERACTIVE_TIMEOUT_SECS,
    )
    .await
}
