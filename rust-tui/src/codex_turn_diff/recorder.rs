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
        pane_id: clean(event.tmux.pane_id.as_deref()),
        repo_root: snapshot.repo_root.to_string_lossy().to_string(),
        cwd: cwd.to_string_lossy().to_string(),
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
    let record = CompletedTurnDiff {
        id: record_id,
        session_id: pending
            .session_id
            .clone()
            .or_else(|| clean(event.session_id.as_deref())),
        turn_id: pending
            .turn_id
            .clone()
            .or_else(|| clean(event.turn_id.as_deref())),
        pane_id: pending
            .pane_id
            .clone()
            .or_else(|| clean(event.tmux.pane_id.as_deref())),
        repo_root: pending.repo_root.clone(),
        cwd: pending.cwd.clone(),
        prompt: pending
            .prompt
            .clone()
            .or_else(|| clean(event.prompt.as_deref())),
        started_at: pending.started_at.clone(),
        ended_at: event_timestamp(event),
        base_tree: pending.base_tree.clone(),
        end_tree: end.tree,
        patch_path: String::new(),
        stats: stats_from_patch(&patch),
    };
    let record = storage::save_completed(record, &patch)?;
    storage::remove_pending(&pending.id)?;
    Ok(Some(record))
}

fn event_cwd(event: &HookEvent) -> Option<PathBuf> {
    clean(event.cwd.as_deref())
        .or_else(|| clean(event.tmux.pane_current_path.as_deref()))
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
