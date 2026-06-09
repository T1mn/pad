use super::app::App;
use super::preview::{FilePreview, PreviewKind};
use crate::codex_turn_diff::{TurnDiffEntry, TurnDiffStatus};

impl App {
    pub fn selected_codex_diff(&self) -> Option<&TurnDiffEntry> {
        self.codex_diffs.get(self.codex_diff_selected)
    }

    pub(crate) fn refresh_codex_diff_preview(&mut self) {
        let previous_scroll = self.file_preview.scroll;
        let selected = self.selected_codex_diff().cloned();
        if let Some(entry) = selected.as_ref() {
            if entry.status == TurnDiffStatus::Completed {
                let key = codex_diff_preview_key(entry);
                if self.codex_diff_preview_key.as_deref() == Some(key.as_str()) {
                    return;
                }
                self.codex_diff_preview_key = Some(key);
            } else {
                self.codex_diff_preview_key = None;
            }
        } else {
            self.codex_diff_preview_key = None;
        }

        let preview = selected
            .as_ref()
            .map(codex_diff_preview)
            .unwrap_or_else(|| {
                FilePreview::new(
                    "codex runs".into(),
                    "No Codex turn diffs recorded for this repo yet.\n\nRun Codex through PAD hooks, then submit a prompt.".into(),
                    PreviewKind::Missing,
                )
        });
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
    let content = format!(
        "Codex turn diff\nstatus: {status}\nstarted: {}\nended: {ended}\nprompt: {prompt}\nfiles: {}  +{}  -{}\n\n{}",
        entry.started_at,
        entry.stats.files_changed,
        entry.stats.insertions,
        entry.stats.deletions,
        patch
    );
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
