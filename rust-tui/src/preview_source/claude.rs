mod line;
mod text;

#[cfg(test)]
mod tests;

use super::turns::{finalize_turns, push_session_message};
use super::SessionReadMode;
use crate::model::PreviewTurn;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::Path;

pub(super) fn parse_transcript(
    path: &Path,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    let mut turns = VecDeque::new();

    line::for_each_session_line(path, read_mode, |line| {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return;
        };
        let Some((role, text)) = text::message_text(&value) else {
            return;
        };
        push_session_message(&mut turns, role, text);
    })
    .map_err(|err| err.to_string())?;

    Ok(finalize_turns(turns))
}
