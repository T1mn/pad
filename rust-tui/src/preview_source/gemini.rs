use super::SessionReadMode;
use crate::model::PreviewTurn;
use std::path::Path;

mod message;
mod read;
mod text;

#[cfg(test)]
mod tests;

pub(super) fn parse_transcript(
    path: &Path,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    match read_mode {
        SessionReadMode::FullBackfill => message::parse_full_transcript(path),
    }
}

pub(super) fn extract_session_id_from_transcript(path: &Path) -> Option<String> {
    read::read_transcript_value(path)
        .ok()
        .and_then(|value| {
            value
                .get("sessionId")
                .and_then(serde_json::Value::as_str)
                .map(|session_id| session_id.trim().to_string())
        })
        .filter(|session_id| !session_id.is_empty())
}
