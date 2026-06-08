pub(in crate::chat::providers::telegram) fn telegram_chat_id_value(
    chat_id: &str,
) -> serde_json::Value {
    chat_id
        .parse::<i64>()
        .map(serde_json::Value::from)
        .unwrap_or_else(|_| serde_json::Value::from(chat_id.to_string()))
}
