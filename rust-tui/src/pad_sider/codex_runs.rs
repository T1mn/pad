use super::app::App;
use super::preview::{FilePreview, PreviewKind};
use crate::codex_turn_diff::{TurnDiffEntry, TurnDiffStatus};
use std::fmt::Write as _;

impl App {
    pub fn selected_codex_diff(&self) -> Option<&TurnDiffEntry> {
        self.codex_diffs.get(self.codex_diff_selected)
    }

    pub(crate) fn refresh_codex_diff_preview(&mut self) {
        let previous_scroll = self.file_preview.scroll;
        let (preview_key, preview) = match self.selected_codex_diff() {
            Some(entry) => {
                if entry.status == TurnDiffStatus::Completed {
                    let key = codex_diff_preview_key(entry);
                    if self.codex_diff_preview_key.as_deref() == Some(key.as_str()) {
                        return;
                    }
                    (Some(key), codex_diff_preview(entry))
                } else {
                    (None, codex_diff_preview(entry))
                }
            }
            None => (
                None,
                FilePreview::new(
                    "codex runs".into(),
                    "No Codex turn diffs recorded for this repo yet.\n\nRun Codex through PAD hooks, then submit a prompt.".into(),
                    PreviewKind::Missing,
                ),
            ),
        };
        self.codex_diff_preview_key = preview_key;
        let mut preview = preview;
        if preview.title == self.file_preview.title {
            preview.scroll = previous_scroll;
        }
        self.set_file_preview(preview);
    }
}

fn codex_diff_preview_key(entry: &TurnDiffEntry) -> String {
    format!(
        "{}|{:?}|{}|{}|{}|{}|{}|{}|{}",
        entry.id,
        entry.status,
        entry.started_at,
        entry.ended_at.as_deref().unwrap_or("-"),
        entry.prompt.as_deref().unwrap_or(""),
        entry.stats.files_changed,
        entry.stats.insertions,
        entry.stats.deletions,
        entry
            .patch_path
            .as_ref()
            .map(|path| path.to_string_lossy())
            .unwrap_or_default()
    )
}

fn codex_diff_preview(entry: &TurnDiffEntry) -> FilePreview {
    let status = match entry.status {
        TurnDiffStatus::Running => "running",
        TurnDiffStatus::Completed => "completed",
    };
    let prompt = crate::codex_turn_diff::prompt_summary(entry.prompt.as_deref(), 120);
    let patch = crate::codex_turn_diff::read_patch(entry)
        .unwrap_or_else(|err| format!("failed to read Codex turn diff: {err}"));
    let patch = if patch.trim().is_empty() {
        "(no file changes in this Codex turn)".to_string()
    } else {
        patch
    };
    let ended = entry.ended_at.as_deref().unwrap_or("-");
    let mut content = String::with_capacity(96 + prompt.len() + patch.len());
    let _ = write!(
        content,
        "Codex turn diff\nstatus: {status}\nstarted: {}\nended: {ended}\nprompt: {prompt}\nfiles: {}  +{}  -{}\n\n",
        entry.started_at,
        entry.stats.files_changed,
        entry.stats.insertions,
        entry.stats.deletions,
    );
    content.push_str(&patch);
    FilePreview::new(
        format!(
            "codex diff {}",
            entry.turn_id.as_deref().unwrap_or(&entry.id)
        ),
        content,
        PreviewKind::Diff,
    )
}

#[cfg(test)]
#[path = "codex_runs_tests.rs"]
mod tests;
