use serde::{Deserialize, Serialize};

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
