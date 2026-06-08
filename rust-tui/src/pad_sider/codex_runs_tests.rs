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
    crate::codex_turn_diff::record_codex_hook_event(&event("stop", &repo, "turn-sider")).unwrap();

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
