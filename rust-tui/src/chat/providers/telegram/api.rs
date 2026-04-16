use super::locale::tg;
use crate::log_debug;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io;
use std::sync::LazyLock;
use std::time::Instant;

const TELEGRAM_TIMEOUT_SECS: u64 = 12;
const TELEGRAM_POLL_TIMEOUT_SECS: u64 = 1;
const TELEGRAM_MAX_TEXT_LEN: usize = 3500;
const TELEGRAM_INTERACTIVE_TIMEOUT_SECS: u64 = 4;
const TELEGRAM_STATUS_TIMEOUT_SECS: u64 = 3;

static TELEGRAM_HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .use_rustls_tls()
        .pool_idle_timeout(std::time::Duration::from_secs(30))
        .tcp_keepalive(std::time::Duration::from_secs(30))
        .user_agent(format!("pad/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("telegram http client")
});

#[derive(Clone, Debug, Deserialize)]
pub(super) struct TelegramEnvelope<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct TelegramUpdate {
    pub(super) update_id: i64,
    pub(super) message: Option<TelegramMessage>,
    pub(super) callback_query: Option<TelegramCallbackQuery>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct TelegramMessage {
    pub(super) message_id: i64,
    pub(super) chat: TelegramChat,
    pub(super) text: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct TelegramCallbackQuery {
    pub(super) id: String,
    pub(super) message: Option<TelegramMessage>,
    pub(super) data: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct TelegramChat {
    pub(super) id: i64,
    #[serde(rename = "type")]
    pub(super) kind: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct TelegramMe {
    pub(super) username: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct TelegramCommandSpec {
    command: &'static str,
    description: String,
}

pub(super) async fn fetch_me(
    token: &str,
) -> Result<TelegramMe, Box<dyn std::error::Error + Send + Sync>> {
    let result = telegram_api::<TelegramMe>(token, "getMe", &json!({}), 15).await?;
    Ok(result)
}

pub(super) async fn get_updates(
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

pub(super) async fn send_text(
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

pub(super) async fn send_chat_action(
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

pub(super) async fn send_message_draft(
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

pub(super) fn telegram_chat_id_value(chat_id: &str) -> serde_json::Value {
    chat_id
        .parse::<i64>()
        .map(serde_json::Value::from)
        .unwrap_or_else(|_| serde_json::Value::from(chat_id.to_string()))
}

pub(super) async fn answer_callback_query(
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

pub(super) async fn set_my_commands(
    token: &str,
    locale: crate::i18n::Locale,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let commands = vec![
        TelegramCommandSpec {
            command: "start",
            description: tg(locale, "command.start").to_string(),
        },
        TelegramCommandSpec {
            command: "help",
            description: tg(locale, "command.help").to_string(),
        },
        TelegramCommandSpec {
            command: "list",
            description: tg(locale, "command.list").to_string(),
        },
        TelegramCommandSpec {
            command: "use",
            description: tg(locale, "command.use").to_string(),
        },
        TelegramCommandSpec {
            command: "history",
            description: tg(locale, "command.history").to_string(),
        },
        TelegramCommandSpec {
            command: "restart",
            description: tg(locale, "command.restart").to_string(),
        },
        TelegramCommandSpec {
            command: "status",
            description: tg(locale, "command.status").to_string(),
        },
        TelegramCommandSpec {
            command: "fast",
            description: tg(locale, "command.fast").to_string(),
        },
        TelegramCommandSpec {
            command: "compact",
            description: tg(locale, "command.compact").to_string(),
        },
        TelegramCommandSpec {
            command: "reset",
            description: tg(locale, "command.reset").to_string(),
        },
        TelegramCommandSpec {
            command: "stop",
            description: tg(locale, "command.stop").to_string(),
        },
    ];
    let _: serde_json::Value = telegram_api(
        token,
        "setMyCommands",
        &json!({
            "commands": commands
        }),
        TELEGRAM_INTERACTIVE_TIMEOUT_SECS,
    )
    .await?;
    Ok(())
}

pub(super) async fn send_message(
    token: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    telegram_api(token, "sendMessage", payload, TELEGRAM_TIMEOUT_SECS).await
}

pub(super) async fn edit_message(
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

async fn telegram_api<T: for<'de> Deserialize<'de>>(
    token: &str,
    method: &str,
    payload: &serde_json::Value,
    timeout_secs: u64,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://api.telegram.org/bot{}/{}", token, method);
    let started_at = Instant::now();
    let response = TELEGRAM_HTTP
        .post(url)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .json(payload)
        .send()
        .await
        .map_err(|err| io::Error::other(format!("telegram {} request failed: {}", method, err)))?;

    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|err| io::Error::other(format!("telegram {} body failed: {}", method, err)))?;

    if !status.is_success() {
        return Err(io::Error::other(format!(
            "telegram {} http {}: {}",
            method,
            status,
            String::from_utf8_lossy(&body).trim()
        ))
        .into());
    }

    let envelope: TelegramEnvelope<T> = serde_json::from_slice(&body)?;
    if !envelope.ok {
        return Err(io::Error::other(
            envelope
                .description
                .unwrap_or_else(|| format!("telegram api {} failed", method)),
        )
        .into());
    }
    log_debug!(
        "telegram_api: method={} timeout_s={} elapsed_ms={}",
        method,
        timeout_secs,
        started_at.elapsed().as_millis()
    );
    envelope.result.ok_or_else(|| {
        io::Error::other(format!("telegram api {} returned no result", method)).into()
    })
}

pub(super) fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut buf = String::new();
    let mut count = 0usize;
    for ch in text.chars() {
        if count >= max_chars {
            chunks.push(std::mem::take(&mut buf));
            count = 0;
        }
        buf.push(ch);
        count += 1;
    }
    if !buf.is_empty() {
        chunks.push(buf);
    }
    chunks
}
