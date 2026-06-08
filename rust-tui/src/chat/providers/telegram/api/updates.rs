use super::client::telegram_api;
use super::types::{TelegramMe, TelegramUpdate};
use super::{TELEGRAM_POLL_TIMEOUT_SECS, TELEGRAM_TIMEOUT_SECS};
use serde_json::json;

pub(in crate::chat::providers::telegram) async fn fetch_me(
    token: &str,
) -> Result<TelegramMe, Box<dyn std::error::Error + Send + Sync>> {
    let result = telegram_api::<TelegramMe>(token, "getMe", &json!({}), 15).await?;
    Ok(result)
}

pub(in crate::chat::providers::telegram) async fn get_updates(
    token: &str,
    offset: i64,
) -> Result<Vec<TelegramUpdate>, Box<dyn std::error::Error + Send + Sync>> {
    telegram_api::<Vec<TelegramUpdate>>(
        token,
        "getUpdates",
        &json!({
            "offset": offset,
            "timeout": TELEGRAM_POLL_TIMEOUT_SECS,
            "allowed_updates": ["message", "callback_query"]
        }),
        TELEGRAM_TIMEOUT_SECS,
    )
    .await
}
