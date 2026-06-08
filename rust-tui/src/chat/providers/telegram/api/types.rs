use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::chat::providers::telegram) struct TelegramEnvelope<T> {
    pub(in crate::chat::providers::telegram) ok: bool,
    pub(in crate::chat::providers::telegram) result: Option<T>,
    pub(in crate::chat::providers::telegram) description: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::chat::providers::telegram) struct TelegramUpdate {
    pub(in crate::chat::providers::telegram) update_id: i64,
    pub(in crate::chat::providers::telegram) message: Option<TelegramMessage>,
    pub(in crate::chat::providers::telegram) callback_query: Option<TelegramCallbackQuery>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::chat::providers::telegram) struct TelegramMessage {
    pub(in crate::chat::providers::telegram) message_id: i64,
    pub(in crate::chat::providers::telegram) chat: TelegramChat,
    pub(in crate::chat::providers::telegram) text: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::chat::providers::telegram) struct TelegramCallbackQuery {
    pub(in crate::chat::providers::telegram) id: String,
    pub(in crate::chat::providers::telegram) message: Option<TelegramMessage>,
    pub(in crate::chat::providers::telegram) data: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::chat::providers::telegram) struct TelegramChat {
    pub(in crate::chat::providers::telegram) id: i64,
    #[serde(rename = "type")]
    pub(in crate::chat::providers::telegram) kind: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::chat::providers::telegram) struct TelegramMe {
    pub(in crate::chat::providers::telegram) username: Option<String>,
}
