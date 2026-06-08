mod db;
mod text;

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
