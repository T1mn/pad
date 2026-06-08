use super::model::TelegramState;
use std::fs;
use std::io;

pub(in crate::chat::providers::telegram) fn load_state() -> io::Result<TelegramState> {
    let path = crate::paths::telegram_state_path();
    match fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(TelegramState::default()),
        Err(err) => Err(err),
    }
}

pub(in crate::chat::providers::telegram) fn save_state(state: &TelegramState) -> io::Result<()> {
    let body = serde_json::to_string_pretty(state)?;
    fs::write(crate::paths::telegram_state_path(), body)
}

pub(in crate::chat::providers::telegram) fn journal_len() -> u64 {
    fs::metadata(crate::paths::hook_events_path())
        .map(|meta| meta.len())
        .unwrap_or(0)
}
