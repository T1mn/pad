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
mod tests {
    use super::{save_state_if_changed, serialized_state};
    use crate::chat::providers::telegram::TelegramState;

    #[test]
    pub(super) fn serialized_state_matches_disk_format() {
        let state = TelegramState::default();
        let body = serialized_state(&state).expect("serialize telegram state");
        assert_eq!(
            body,
            serde_json::to_string_pretty(&state).expect("serialize reference")
        );
    }

    #[test]
    pub(super) fn save_state_if_changed_skips_identical_body() {
        let state = TelegramState::default();
        let mut last_saved = Some(serialized_state(&state).expect("serialize initial state"));

        let changed = save_state_if_changed(&state, &mut last_saved).expect("save if changed");
        assert!(!changed);
    }
}
