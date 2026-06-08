use super::json::{read_json, read_json_dir, write_json};
use super::paths::{event_key, pending_dir, pending_path};
use crate::codex_turn_diff::model::PendingTurnDiff;
use crate::hook::HookEvent;
use std::fs;
use std::io;

pub fn save_pending(pending: &PendingTurnDiff) -> io::Result<()> {
    fs::create_dir_all(pending_dir())?;
    write_json(&pending_path(&pending.id), pending)
}

pub fn load_pending_for_stop(event: &HookEvent) -> io::Result<Option<PendingTurnDiff>> {
    if let Some(key) = event_key(event) {
        let path = pending_path(&key);
        if path.exists() {
            return read_json(&path).map(Some);
        }
    }

    let mut candidates = list_pending_all()?
        .into_iter()
        .filter(|pending| pending_matches_event(pending, event))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    Ok(candidates.into_iter().next())
}

pub fn remove_pending(id: &str) -> io::Result<()> {
    let path = pending_path(id);
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

pub(super) fn list_pending_all() -> io::Result<Vec<PendingTurnDiff>> {
    read_json_dir(&pending_dir())
}

fn pending_matches_event(pending: &PendingTurnDiff, event: &HookEvent) -> bool {
    if let Some(turn_id) = event.turn_id.as_deref().filter(|value| !value.is_empty()) {
        return pending.turn_id.as_deref() == Some(turn_id);
    }
    if let Some(session_id) = event
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        if pending.session_id.as_deref() != Some(session_id) {
            return false;
        }
    }
    if let Some(pane_id) = event
        .tmux
        .pane_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        if pending.pane_id.as_deref() != Some(pane_id) {
            return false;
        }
    }
    event.session_id.is_some() || event.tmux.pane_id.is_some()
}
