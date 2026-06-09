#[path = "codex/normalize.rs"]
mod normalize;
#[path = "codex/parser.rs"]
mod parser;
#[path = "codex/status_probe.rs"]
mod status_probe;
#[path = "codex/subagent.rs"]
mod subagent;
#[path = "codex/tail.rs"]
mod tail;

use super::SessionReadMode;
use crate::model::PreviewTurn;
use std::path::Path;

pub(crate) use normalize::{normalize_codex_user_text, normalize_codex_user_text_cow};

pub(super) fn parse_transcript(
    path: &Path,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    parser::parse_transcript(path, read_mode)
}

pub(super) fn resolve_live_session_id(pane_id: &str) -> Option<String> {
    status_probe::resolve_live_session_id(pane_id)
}

#[cfg(test)]
#[path = "codex/tests.rs"]
mod tests;
