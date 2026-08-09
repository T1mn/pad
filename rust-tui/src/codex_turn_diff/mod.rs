mod git {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct GitTreeSnapshot {
        pub repo_root: PathBuf,
        pub tree: String,
    }

    pub fn repo_root_for_cwd(cwd: &Path) -> io::Result<PathBuf> {
        let output = git_output(cwd, &["rev-parse", "--show-toplevel"])?;
        let root = PathBuf::from(output.trim());
        fs::canonicalize(&root).or(Ok(root))
    }

    pub fn capture_worktree_tree(cwd: &Path) -> io::Result<GitTreeSnapshot> {
        let repo_root = repo_root_for_cwd(cwd)?;
        let index_path = temp_index_path();

        let result = (|| {
            if head_exists(&repo_root)? {
                git_output_with_index(&repo_root, &index_path, &["read-tree", "HEAD"])?;
            }
            git_output_with_index(&repo_root, &index_path, &["add", "-A", "--", "."])?;
            let tree = git_output_with_index(&repo_root, &index_path, &["write-tree"])?;
            Ok(GitTreeSnapshot {
                repo_root,
                tree: tree.trim().to_string(),
            })
        })();

        let _ = fs::remove_file(&index_path);
        let _ = fs::remove_file(index_path.with_extension("lock"));
        result
    }

    pub fn diff_trees(repo_root: &Path, base_tree: &str, end_tree: &str) -> io::Result<String> {
        git_output(
            repo_root,
            &[
                "diff",
                "--no-ext-diff",
                "--find-renames",
                "--src-prefix=a/",
                "--dst-prefix=b/",
                base_tree,
                end_tree,
            ],
        )
    }

    fn head_exists(repo_root: &Path) -> io::Result<bool> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .args(["rev-parse", "--verify", "HEAD^{tree}"])
            .output()?;
        Ok(output.status.success())
    }

    fn git_output(cwd: &Path, args: &[&str]) -> io::Result<String> {
        let output = Command::new("git").arg("-C").arg(cwd).args(args).output()?;
        command_output(output, args)
    }

    fn git_output_with_index(cwd: &Path, index_path: &Path, args: &[&str]) -> io::Result<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .env("GIT_INDEX_FILE", index_path)
            .args(args)
            .output()?;
        command_output(output, args)
    }

    fn command_output(output: std::process::Output, args: &[&str]) -> io::Result<String> {
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        Err(io::Error::other(format!(
            "git {} failed: {}",
            format_git_args(args),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }

    fn temp_index_path() -> PathBuf {
        let stamp = crate::time::unix_now_nanos();
        std::env::temp_dir().join(format!(
            "pad-codex-turn-diff-{}-{stamp}.index",
            std::process::id()
        ))
    }

    pub(super) fn format_git_args(args: &[&str]) -> String {
        let mut formatted = String::new();
        for arg in args {
            if !formatted.is_empty() {
                formatted.push(' ');
            }
            formatted.push_str(arg);
        }
        formatted
    }
}
mod model {
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
}
mod recorder {
    use super::git::{capture_worktree_tree, diff_trees};
    use super::model::{stats_from_patch, CompletedTurnDiff, PendingTurnDiff};
    use super::storage;
    use crate::hook::HookEvent;
    use std::io;
    use std::path::{Path, PathBuf};

    pub fn record_codex_hook_event(event: &HookEvent) -> io::Result<Option<CompletedTurnDiff>> {
        match event.event.as_str() {
            "user_prompt_submit" => {
                begin_turn(event)?;
                Ok(None)
            }
            "stop" => finish_turn(event),
            _ => Ok(None),
        }
    }

    fn begin_turn(event: &HookEvent) -> io::Result<Option<PendingTurnDiff>> {
        let Some(key) = storage::event_key(event) else {
            return Ok(None);
        };
        let Some(cwd) = event_cwd(event) else {
            return Ok(None);
        };
        let Ok(snapshot) = capture_worktree_tree(&cwd) else {
            return Ok(None);
        };

        let pending = PendingTurnDiff {
            id: key,
            session_id: clean(event.session_id.as_deref()),
            turn_id: clean(event.turn_id.as_deref()),
            pane_id: clean(event.terminal.pane_id.as_deref()),
            repo_root: snapshot.repo_root.to_string_lossy().into_owned(),
            cwd: cwd.to_string_lossy().into_owned(),
            prompt: clean(event.prompt.as_deref()),
            started_at: event_timestamp(event),
            base_tree: snapshot.tree,
        };
        storage::save_pending(&pending)?;
        Ok(Some(pending))
    }

    fn finish_turn(event: &HookEvent) -> io::Result<Option<CompletedTurnDiff>> {
        let Some(pending) = storage::load_pending_for_stop(event)? else {
            return Ok(None);
        };
        let end = capture_worktree_tree(Path::new(&pending.repo_root))?;
        let patch = diff_trees(&end.repo_root, &pending.base_tree, &end.tree)?;
        let record_id = storage::new_record_id(&pending.id);
        let PendingTurnDiff {
            id: pending_id,
            session_id,
            turn_id,
            pane_id,
            repo_root,
            cwd,
            prompt,
            started_at,
            base_tree,
            ..
        } = pending;
        let record = CompletedTurnDiff {
            id: record_id,
            session_id: session_id.or_else(|| clean(event.session_id.as_deref())),
            turn_id: turn_id.or_else(|| clean(event.turn_id.as_deref())),
            pane_id: pane_id.or_else(|| clean(event.terminal.pane_id.as_deref())),
            repo_root,
            cwd,
            prompt: prompt.or_else(|| clean(event.prompt.as_deref())),
            started_at,
            ended_at: event_timestamp(event),
            base_tree,
            end_tree: end.tree,
            patch_path: String::new(),
            stats: stats_from_patch(&patch),
        };
        let record = storage::save_completed(record, &patch)?;
        storage::remove_pending(&pending_id)?;
        Ok(Some(record))
    }

    fn event_cwd(event: &HookEvent) -> Option<PathBuf> {
        clean(event.cwd.as_deref())
            .or_else(|| clean(event.terminal.pane_current_path.as_deref()))
            .map(PathBuf::from)
    }

    fn event_timestamp(event: &HookEvent) -> String {
        clean(event.timestamp.as_deref()).unwrap_or_else(storage::now_stamp)
    }

    fn clean(value: Option<&str>) -> Option<String> {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }
}
mod storage;
mod storage_paths {
    use crate::hook::HookEvent;
    use std::path::PathBuf;

    const STORE_DIR: &str = "codex-turn-diffs";

    pub fn storage_root() -> PathBuf {
        if let Some(path) = std::env::var_os("PAD_CODEX_TURN_DIFF_DIR") {
            return PathBuf::from(path);
        }
        crate::paths::pad_home_dir().join(STORE_DIR)
    }

    pub fn pending_dir() -> PathBuf {
        storage_root().join("pending")
    }

    pub fn records_dir() -> PathBuf {
        storage_root().join("records")
    }

    pub fn patches_dir() -> PathBuf {
        storage_root().join("patches")
    }

    pub fn index_path() -> PathBuf {
        storage_root().join("index.jsonl")
    }

    pub fn pending_path(id: &str) -> PathBuf {
        pending_dir().join(format!("{}.json", safe_name(id)))
    }

    pub fn record_path(id: &str) -> PathBuf {
        records_dir().join(format!("{}.json", safe_name(id)))
    }

    pub fn new_record_id(key: &str) -> String {
        format!("{}_{}", now_nanos(), safe_name(key))
    }

    pub fn event_key(event: &HookEvent) -> Option<String> {
        if let Some(turn_id) = event.turn_id.as_deref().filter(|value| !value.is_empty()) {
            return Some(format!("turn_{}", safe_name(turn_id)));
        }
        match (
            event
                .session_id
                .as_deref()
                .filter(|value| !value.is_empty()),
            event
                .terminal
                .pane_id
                .as_deref()
                .filter(|value| !value.is_empty()),
        ) {
            (Some(session_id), Some(pane_id)) => Some(format!(
                "session_{}_pane_{}",
                safe_name(session_id),
                safe_name(pane_id)
            )),
            (Some(session_id), None) => Some(format!("session_{}", safe_name(session_id))),
            (None, Some(pane_id)) => Some(format!("pane_{}", safe_name(pane_id))),
            (None, None) => None,
        }
    }

    pub fn now_stamp() -> String {
        crate::time::unix_now_secs().to_string()
    }

    pub(super) fn safe_name(value: &str) -> String {
        let mut out = String::with_capacity(value.len().min(96));
        for ch in value.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                if out.is_empty() && ch == '_' {
                    continue;
                }
                out.push(ch);
            } else if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
        }
        while out.ends_with('_') {
            out.pop();
        }
        out.truncate(out.len().min(96));
        out
    }

    fn now_nanos() -> u128 {
        crate::time::unix_now_nanos()
    }
}

pub use recorder::record_codex_hook_event;

use crate::hook::HookEvent;
use std::io::{self, Read};

pub fn run_args<I>(mut args: I) -> Result<(), Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        Some("hook") => {
            let mut raw = String::new();
            io::stdin().read_to_string(&mut raw)?;
            let event: HookEvent = serde_json::from_str(&raw)?;
            record_codex_hook_event(&event)?;
            Ok(())
        }
        Some(other) => Err(format!("unknown codex-turn-diff command: {other}").into()),
        None => Err("usage: pad __internal codex-turn-diff hook < event.json".into()),
    }
}

#[cfg(test)]
pub(crate) mod tests;
