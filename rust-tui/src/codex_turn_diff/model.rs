use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingTurnDiff {
    pub id: String,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub pane_id: Option<String>,
    pub repo_root: String,
    pub cwd: String,
    pub prompt: Option<String>,
    pub started_at: String,
    pub base_tree: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletedTurnDiff {
    pub id: String,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub pane_id: Option<String>,
    pub repo_root: String,
    pub cwd: String,
    pub prompt: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub base_tree: String,
    pub end_tree: String,
    pub patch_path: String,
    pub stats: DiffStats,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnDiffStatus {
    Running,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnDiffEntry {
    pub id: String,
    pub status: TurnDiffStatus,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub pane_id: Option<String>,
    pub repo_root: PathBuf,
    pub cwd: PathBuf,
    pub prompt: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub base_tree: String,
    pub patch_path: Option<PathBuf>,
    pub stats: DiffStats,
}

impl TurnDiffEntry {
    pub fn sort_key(&self) -> String {
        self.ended_at
            .clone()
            .unwrap_or_else(|| self.started_at.clone())
    }
}

impl From<PendingTurnDiff> for TurnDiffEntry {
    fn from(value: PendingTurnDiff) -> Self {
        Self {
            id: value.id,
            status: TurnDiffStatus::Running,
            session_id: value.session_id,
            turn_id: value.turn_id,
            pane_id: value.pane_id,
            repo_root: PathBuf::from(value.repo_root),
            cwd: PathBuf::from(value.cwd),
            prompt: value.prompt,
            started_at: value.started_at,
            ended_at: None,
            base_tree: value.base_tree,
            patch_path: None,
            stats: DiffStats::default(),
        }
    }
}

impl From<CompletedTurnDiff> for TurnDiffEntry {
    fn from(value: CompletedTurnDiff) -> Self {
        Self {
            id: value.id,
            status: TurnDiffStatus::Completed,
            session_id: value.session_id,
            turn_id: value.turn_id,
            pane_id: value.pane_id,
            repo_root: PathBuf::from(value.repo_root),
            cwd: PathBuf::from(value.cwd),
            prompt: value.prompt,
            started_at: value.started_at,
            ended_at: Some(value.ended_at),
            base_tree: value.base_tree,
            patch_path: Some(PathBuf::from(value.patch_path)),
            stats: value.stats,
        }
    }
}

pub fn stats_from_patch(patch: &str) -> DiffStats {
    let mut stats = DiffStats::default();
    for line in patch.lines() {
        if line.starts_with("diff --git ") {
            stats.files_changed += 1;
        } else if line.starts_with('+') && !line.starts_with("+++") {
            stats.insertions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            stats.deletions += 1;
        }
    }
    stats
}

pub fn prompt_summary(prompt: Option<&str>, max_chars: usize) -> String {
    let text = prompt
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("(no prompt)");
    let mut out = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars {
            out.push('…');
            break;
        }
        if ch.is_whitespace() {
            if !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.push(ch);
        }
    }
    out.trim().to_string()
}
