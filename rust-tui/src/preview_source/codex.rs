#[path = "codex/normalize.rs"]
mod normalize;
#[path = "codex/parser.rs"]
mod parser;
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

#[cfg(test)]
#[path = "codex/tests.rs"]
mod tests;
