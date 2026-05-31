use super::app::App;
use super::preview::{FilePreview, PreviewKind};
use crate::codex_turn_diff::{TurnDiffEntry, TurnDiffStatus};

impl App {
    pub fn selected_codex_diff(&self) -> Option<&TurnDiffEntry> {
        self.codex_diffs.get(self.codex_diff_selected)
    }

    pub(crate) fn refresh_codex_diff_preview(&mut self) {
        let previous_title = self.file_preview.title.clone();
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
        if preview.title == previous_title {
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
mod tests {
    use super::*;
    use crate::hook::{HookEvent, HookTmuxInfo};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn app_codex_runs_mode_previews_recorded_turn_patch() {
        if !git_available() {
            eprintln!("git unavailable; skipping pad_sider codex runs test");
            return;
        }
        let _guard = crate::test_support::home_env_lock()
            .lock()
            .expect("env lock");
        let root = temp_dir("pad-sider-codex-runs");
        let repo = root.join("repo");
        let store = root.join("store");
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&store).unwrap();
        let previous = std::env::var_os("PAD_CODEX_TURN_DIFF_DIR");
        std::env::set_var("PAD_CODEX_TURN_DIFF_DIR", &store);

        git(&repo, &["init"]);
        crate::codex_turn_diff::record_codex_hook_event(&event(
            "user_prompt_submit",
            &repo,
            "turn-sider",
        ))
        .unwrap();
        fs::write(repo.join("sider.txt"), "after\n").unwrap();
        crate::codex_turn_diff::record_codex_hook_event(&event("stop", &repo, "turn-sider"))
            .unwrap();

        let mut app = App::new(repo.clone(), None);
        app.focus_codex_runs();

        assert!(matches!(app.file_preview.kind, PreviewKind::Diff));
        assert!(app.file_preview.content.contains("Codex turn diff"));
        assert!(app.file_preview.content.contains("+after"));

        if let Some(previous) = previous {
            std::env::set_var("PAD_CODEX_TURN_DIFF_DIR", previous);
        } else {
            std::env::remove_var("PAD_CODEX_TURN_DIFF_DIR");
        }
        let _ = fs::remove_dir_all(root);
    }

    fn event(kind: &str, repo: &Path, turn_id: &str) -> HookEvent {
        HookEvent {
            event: kind.into(),
            turn_id: Some(turn_id.into()),
            session_id: Some("session-sider".into()),
            transcript_path: None,
            cwd: Some(repo.to_string_lossy().to_string()),
            prompt: Some("show in sider".into()),
            last_assistant_message: None,
            timestamp: Some("2026-05-31T00:00:00Z".into()),
            tmux: HookTmuxInfo {
                pane_id: Some("%9".into()),
                session_name: Some("pad".into()),
                window_index: Some("0".into()),
                pane_index: Some("0".into()),
                pane_current_path: Some(repo.to_string_lossy().to_string()),
            },
        }
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("pad-{name}-{stamp}"))
    }
}
