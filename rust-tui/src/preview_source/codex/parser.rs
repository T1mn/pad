#[path = "parser/function_call.rs"]
mod function_call;
#[path = "parser/lines.rs"]
mod lines;
#[path = "parser/message.rs"]
mod message;
#[path = "parser/model.rs"]
mod model;

use super::tail;
use crate::model::PreviewTurn;
use crate::preview_source::SessionReadMode;
use std::path::Path;

pub(super) fn parse_transcript(
    path: &Path,
    _read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    parse_recent_transcript(path).map_err(|err| err.to_string())
}

fn parse_recent_transcript(path: &Path) -> std::io::Result<Vec<PreviewTurn>> {
    let file_len = tail::file_len(path)?;
    if file_len == 0 {
        return Ok(Vec::new());
    }

    let mut tail_bytes = tail::initial_tail_bytes(file_len);
    loop {
        let lines = tail::read_tail_lines(path, file_len, tail_bytes)?;
        let turns = lines::parse_transcript_lines(lines.iter().map(String::as_str));
        if turns.len() >= crate::session_cache::SESSION_HISTORY_TURN_LIMIT || tail_bytes >= file_len
        {
            return Ok(turns);
        }
        tail_bytes = tail::grow_tail_bytes(tail_bytes, file_len);
    }
}
