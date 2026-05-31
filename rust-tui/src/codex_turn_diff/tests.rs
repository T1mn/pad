use super::*;
use crate::hook::{HookEvent, HookTmuxInfo};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn records_only_changes_between_submit_and_stop() {
    with_isolated_store("single-turn", |repo| {
        git(repo, &["init"]);
        git(repo, &["config", "user.email", "tim@example.com"]);
        git(repo, &["config", "user.name", "Tim"]);
        fs::write(repo.join("app.rs"), "base\n").unwrap();
        git(repo, &["add", "."]);
        git(repo, &["commit", "-m", "init"]);

        fs::write(repo.join("app.rs"), "preexisting dirty\n").unwrap();
        record_codex_hook_event(&event("user_prompt_submit", repo, "turn-1", Some("fix it")))
            .unwrap();

        fs::write(repo.join("app.rs"), "codex result\n").unwrap();
        let record = record_codex_hook_event(&event("stop", repo, "turn-1", None))
            .unwrap()
            .expect("completed record");

        let patch = fs::read_to_string(&record.patch_path).unwrap();
        assert!(patch.contains("-preexisting dirty"));
        assert!(patch.contains("+codex result"));
        assert!(!patch.contains("-base"));
        assert_eq!(record.stats.files_changed, 1);
        assert_eq!(record.stats.insertions, 1);
        assert_eq!(record.stats.deletions, 1);

        let entries = list_for_cwd(repo);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].turn_id.as_deref(), Some("turn-1"));
        assert_eq!(read_patch(&entries[0]).unwrap(), patch);
    });
}

#[test]
fn records_untracked_files_created_by_turn() {
    with_isolated_store("untracked", |repo| {
        git(repo, &["init"]);
        record_codex_hook_event(&event(
            "user_prompt_submit",
            repo,
            "turn-new",
            Some("add file"),
        ))
        .unwrap();

        fs::write(repo.join("new.txt"), "hello\n").unwrap();
        let record = record_codex_hook_event(&event("stop", repo, "turn-new", None))
            .unwrap()
            .expect("completed record");

        let patch = fs::read_to_string(record.patch_path).unwrap();
        assert!(patch.contains("diff --git a/new.txt b/new.txt"));
        assert!(patch.contains("new file mode"));
        assert!(patch.contains("+hello"));
    });
}

#[test]
fn running_entry_reads_live_diff_from_pending_baseline() {
    with_isolated_store("live", |repo| {
        git(repo, &["init"]);
        fs::write(repo.join("live.txt"), "before\n").unwrap();
        record_codex_hook_event(&event(
            "user_prompt_submit",
            repo,
            "turn-live",
            Some("live"),
        ))
        .unwrap();

        fs::write(repo.join("live.txt"), "after\n").unwrap();
        let entries = list_for_cwd(repo);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, TurnDiffStatus::Running);

        let patch = read_patch(&entries[0]).unwrap();
        assert!(patch.contains("-before"));
        assert!(patch.contains("+after"));
    });
}

#[test]
fn cli_hook_records_from_normalized_hook_json() {
    with_isolated_store("cli", |repo| {
        git(repo, &["init"]);
        let submit =
            serde_json::to_string(&event("user_prompt_submit", repo, "turn-cli", None)).unwrap();
        let parsed: HookEvent = serde_json::from_str(&submit).unwrap();
        record_codex_hook_event(&parsed).unwrap();
        fs::write(repo.join("cli.txt"), "ok\n").unwrap();
        let stop = event("stop", repo, "turn-cli", None);
        assert!(record_codex_hook_event(&stop).unwrap().is_some());
    });
}

fn with_isolated_store(name: &str, f: impl FnOnce(&Path)) {
    if !git_available() {
        eprintln!("git unavailable; skipping codex_turn_diff test");
        return;
    }

    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("env lock");
    let root = temp_dir(name);
    let repo = root.join("repo");
    let store = root.join("store");
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(&store).unwrap();

    let previous = std::env::var_os("PAD_CODEX_TURN_DIFF_DIR");
    std::env::set_var("PAD_CODEX_TURN_DIFF_DIR", &store);
    f(&repo);
    if let Some(previous) = previous {
        std::env::set_var("PAD_CODEX_TURN_DIFF_DIR", previous);
    } else {
        std::env::remove_var("PAD_CODEX_TURN_DIFF_DIR");
    }
    let _ = fs::remove_dir_all(root);
}

fn event(kind: &str, repo: &Path, turn_id: &str, prompt: Option<&str>) -> HookEvent {
    HookEvent {
        event: kind.into(),
        turn_id: Some(turn_id.into()),
        session_id: Some("session-1".into()),
        transcript_path: Some(repo.join("transcript.jsonl").to_string_lossy().to_string()),
        cwd: Some(repo.to_string_lossy().to_string()),
        prompt: prompt.map(str::to_string),
        last_assistant_message: None,
        timestamp: Some(format!("2026-05-31T00:00:{}Z", turn_id.len())),
        tmux: HookTmuxInfo {
            pane_id: Some("%1".into()),
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
    std::env::temp_dir().join(format!("pad-codex-turn-diff-{name}-{stamp}"))
}
