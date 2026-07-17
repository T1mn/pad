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
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

pub(super) fn parse_transcript(
    path: &Path,
    _read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    parse_recent_transcript(path).map_err(|err| err.to_string())
}

fn parse_recent_transcript(path: &Path) -> std::io::Result<Vec<PreviewTurn>> {
    let rollout_path = resolve_rollout_path(path)?;
    if crate::codex_rollout::is_compressed_rollout(&rollout_path) {
        return parse_compressed_transcript(&rollout_path);
    }

    parse_plain_transcript(&rollout_path)
}

fn parse_plain_transcript(path: &Path) -> io::Result<Vec<PreviewTurn>> {
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

fn parse_compressed_transcript(path: &Path) -> io::Result<Vec<PreviewTurn>> {
    let file = File::open(path)?;
    let decoder = zstd::stream::read::Decoder::new(file)?;
    lines::parse_transcript_reader(BufReader::new(decoder))
}

fn resolve_rollout_path(path: &Path) -> io::Result<PathBuf> {
    crate::codex_rollout::existing_rollout_path(path).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("rollout file not found: {}", path.display()),
        )
    })
}
