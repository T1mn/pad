mod db;
mod text {
    use crate::opencode_text::{
        extract_display_part_text, message_role as parse_message_role, OpenCodeRole,
    };
    use crate::preview_source::turns::SessionRole;

    pub(super) fn message_role(raw: &str) -> Option<SessionRole> {
        match parse_message_role(raw)? {
            OpenCodeRole::User => Some(SessionRole::User),
            OpenCodeRole::Assistant => Some(SessionRole::Assistant),
        }
    }

    pub(super) fn extract_part_text(raw: &str) -> Option<String> {
        extract_display_part_text(raw)
    }
}

use super::SessionReadMode;
use crate::model::PreviewTurn;
use std::path::Path;

pub(super) fn parse_transcript(
    db_path: &Path,
    session_id: Option<&str>,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    let Some(session_id) = session_id else {
        return Ok(Vec::new());
    };

    match read_mode {
        SessionReadMode::FullBackfill => db::parse_session(db_path, session_id),
    }
}

#[cfg(test)]
mod tests;
