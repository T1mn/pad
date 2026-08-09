#[path = "codex/normalize.rs"]
mod normalize;
#[path = "codex/parser.rs"]
mod parser;
#[path = "codex/subagent.rs"]
mod subagent {
    use serde_json::Value;

    pub(super) fn extract_subagent_notification_summary(text: &str) -> Option<String> {
        const OPEN: &str = "<subagent_notification>";
        const CLOSE: &str = "</subagent_notification>";

        let start = text.find(OPEN)?;
        let rest = &text[start + OPEN.len()..];
        let end = rest.find(CLOSE)?;
        let json = rest[..end].trim();
        let value = serde_json::from_str::<Value>(json).ok()?;

        let agent_path = value
            .get("agent_path")
            .and_then(Value::as_str)
            .unwrap_or("subagent");
        let agent_label = agent_path.rsplit('/').next().unwrap_or(agent_path);
        let status = value.get("status").and_then(Value::as_object);
        let (status_label, detail) = if let Some(status) = status {
            if let Some(completed) = status.get("completed").and_then(Value::as_str) {
                ("completed", completed)
            } else if let Some(failed) = status.get("failed").and_then(Value::as_str) {
                ("failed", failed)
            } else if let Some(running) = status.get("running").and_then(Value::as_str) {
                ("running", running)
            } else {
                ("updated", "")
            }
        } else {
            ("updated", "")
        };

        let compact = compact_subagent_detail(detail);
        if compact.is_empty() {
            Some(format!("[subagent/{}] {}", status_label, agent_label))
        } else {
            Some(format!(
                "[subagent/{}] {}\n{}",
                status_label, agent_label, compact
            ))
        }
    }

    fn compact_subagent_detail(text: &str) -> String {
        let line = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim();
        if line.is_empty() {
            return String::new();
        }

        let compact = compact_whitespace(line);
        truncate_chars_with_ellipsis(&compact, 220)
    }

    fn compact_whitespace(line: &str) -> String {
        let mut compact = String::with_capacity(line.len());
        for word in line.split_whitespace() {
            if !compact.is_empty() {
                compact.push(' ');
            }
            compact.push_str(word);
        }
        compact
    }

    fn truncate_chars_with_ellipsis(text: &str, max_chars: usize) -> String {
        if text.chars().count() <= max_chars {
            return text.to_string();
        }

        let mut out = String::new();
        for ch in text.chars().take(max_chars.saturating_sub(1)) {
            out.push(ch);
        }
        out.push('…');
        out
    }
}
#[path = "codex/tail.rs"]
mod tail {
    use std::fs::File;
    use std::io::{self, Read, Seek, SeekFrom};
    use std::path::Path;

    #[cfg(not(test))]
    const INITIAL_TAIL_BYTES: u64 = 2 * 1024 * 1024;
    #[cfg(test)]
    const INITIAL_TAIL_BYTES: u64 = 256;

    pub(super) fn file_len(path: &Path) -> io::Result<u64> {
        std::fs::metadata(path).map(|metadata| metadata.len())
    }

    pub(super) fn initial_tail_bytes(file_len: u64) -> u64 {
        INITIAL_TAIL_BYTES.min(file_len).max(1)
    }

    pub(super) fn grow_tail_bytes(current: u64, file_len: u64) -> u64 {
        current.saturating_mul(2).min(file_len)
    }

    pub(super) fn read_tail_lines(
        path: &Path,
        file_len: u64,
        tail_bytes: u64,
    ) -> io::Result<Vec<String>> {
        let start = file_len.saturating_sub(tail_bytes);
        let mut file = File::open(path)?;
        let read_start = start.saturating_sub(1);
        file.seek(SeekFrom::Start(read_start))?;

        let capacity = tail_bytes
            .saturating_add(u64::from(start > 0))
            .min(usize::MAX as u64) as usize;
        let mut bytes = Vec::with_capacity(capacity);
        file.read_to_end(&mut bytes)?;
        if start > 0 {
            if bytes.first() == Some(&b'\n') {
                bytes.drain(..1);
            } else if let Some(pos) = bytes.iter().position(|byte| *byte == b'\n') {
                bytes.drain(..=pos);
            } else {
                bytes.clear();
            }
        }

        Ok(String::from_utf8_lossy(&bytes)
            .lines()
            .map(str::to_string)
            .collect())
    }
}

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
pub(crate) mod tests;
