use super::super::*;

pub(super) fn serialized_state(state: &TelegramState) -> io::Result<String> {
    serde_json::to_string_pretty(state)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

pub(super) fn save_state_if_changed(
    state: &TelegramState,
    last_saved_state: &mut Option<String>,
) -> io::Result<bool> {
    let body = serialized_state(state)?;
    if last_saved_state.as_deref() == Some(body.as_str()) {
        return Ok(false);
    }
    std::fs::write(crate::paths::telegram_state_path(), &body)?;
    *last_saved_state = Some(body);
    Ok(true)
}

#[cfg(test)]
#[path = "state_io_tests.rs"]
mod tests;
