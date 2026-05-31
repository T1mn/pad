mod git;
mod model;
mod recorder;
mod storage;
mod storage_paths;

pub use git::diff_pending_to_worktree;
pub use model::{prompt_summary, TurnDiffEntry, TurnDiffStatus};
pub use recorder::record_codex_hook_event;

use crate::hook::HookEvent;
use std::io::{self, Read};
use std::path::Path;

const LIST_LIMIT: usize = 40;

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

pub fn list_for_cwd(cwd: &Path) -> Vec<TurnDiffEntry> {
    storage::list_for_cwd(cwd, LIST_LIMIT).unwrap_or_default()
}

pub fn read_patch(entry: &TurnDiffEntry) -> io::Result<String> {
    match entry.status {
        TurnDiffStatus::Completed => {
            let Some(path) = entry.patch_path.as_ref() else {
                return Ok(String::from("Missing patch path"));
            };
            std::fs::read_to_string(path)
        }
        TurnDiffStatus::Running => diff_pending_to_worktree(&entry.repo_root, &entry.base_tree),
    }
}

#[cfg(test)]
mod tests;
