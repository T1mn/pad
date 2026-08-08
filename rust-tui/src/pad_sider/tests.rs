mod fs {
    use super::super::fs::read_text_file;
    use crate::test_support;
    use std::fs;

    #[test]
    fn read_text_file_returns_full_file_without_preview_truncation() {
        let path = temp_file("full_file");
        let body = "x".repeat(260 * 1024);
        fs::write(&path, &body).unwrap();

        let read = read_text_file(&path);

        assert_eq!(read.len(), body.len());
        assert!(!read.contains("truncated preview"));
        fs::remove_file(path).unwrap();
    }

    fn temp_file(name: &str) -> std::path::PathBuf {
        test_support::temp_path("pad_sider_fs", name)
    }
}

mod sizing {
    use super::super::sizing::{default_width, nearest_width_level, next_width_level};

    #[test]
    fn default_width_is_half() {
        assert_eq!(default_width(), "50%");
    }

    #[test]
    fn width_levels_step_without_wrapping() {
        assert_eq!(next_width_level(50, true), 55);
        assert_eq!(next_width_level(65, true), 65);
        assert_eq!(next_width_level(50, false), 45);
        assert_eq!(next_width_level(45, false), 45);
    }

    #[test]
    fn nearest_width_level_handles_tmux_rounding() {
        assert_eq!(nearest_width_level(49), 50);
        assert_eq!(nearest_width_level(51), 50);
        assert_eq!(nearest_width_level(64), 65);
    }
}

mod index_map {
    use super::super::index_map::build_index_map;
    use crate::test_support;
    use std::fs;

    #[test]
    fn builds_nested_index_map_and_skips_ignored_dirs() {
        let root = temp_dir("builds_nested_index_map_and_skips_ignored_dirs");
        fs::create_dir_all(root.join("docs/guide")).unwrap();
        fs::create_dir_all(root.join("target/hidden")).unwrap();
        fs::write(root.join("index.md"), "# root").unwrap();
        fs::write(root.join("docs/index.md"), "# docs").unwrap();
        fs::write(root.join("docs/guide/index.md"), "# guide").unwrap();
        fs::write(root.join("target/hidden/index.md"), "# hidden").unwrap();

        let rows = build_index_map(&root);
        let labels = rows
            .iter()
            .map(|row| (row.depth, row.dir_label.as_str()))
            .collect::<Vec<_>>();

        assert_eq!(labels, vec![(0, "."), (1, "docs"), (2, "guide")]);
        assert!(!rows
            .iter()
            .any(|row| row.path.ends_with("target/hidden/index.md")));

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        test_support::temp_path("pad_sider_index_map", name)
    }
}

mod tmux_args {
    use super::super::tmux_args::format_tmux_args;

    #[test]
    fn tmux_args_format_keeps_sider_error_shape() {
        assert_eq!(
            format_tmux_args(&["display-message", "-p", "#{window_width}"]),
            "display-message -p #{window_width}"
        );
    }
}

mod tmux {
    use super::super::tmux::is_codex_command;

    #[test]
    fn codex_command_detection_is_ascii_case_insensitive() {
        assert!(is_codex_command("codex"));
        assert!(is_codex_command("CODEX"));
        assert!(is_codex_command("/opt/bin/pad-codex"));
        assert!(!is_codex_command("claude"));
    }
}

mod codex_runs {
    use super::super::app::App;
    use super::super::preview::PreviewKind;
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

mod preview_render_cache {
    use super::super::app::App;
    use super::super::preview::{FilePreview, PreviewKind};
    use ratatui::text::Line;

    #[test]
    fn set_file_preview_keeps_render_cache_when_only_scroll_changes() {
        let mut app = App::new(std::env::temp_dir(), None);
        app.set_file_preview(FilePreview::new(
            "a".into(),
            "body".into(),
            PreviewKind::Text,
        ));
        app.store_rendered_file_preview(80, vec![Line::from("body")]);

        let mut next = FilePreview::new("a".into(), "body".into(), PreviewKind::Text);
        next.scroll = 10;
        let revision = app.file_preview_revision;
        app.set_file_preview(next);

        assert_eq!(app.file_preview_revision, revision);
        assert!(app.rendered_file_preview_matches(80));
    }

    #[test]
    fn set_file_preview_invalidates_render_cache_when_content_changes() {
        let mut app = App::new(std::env::temp_dir(), None);
        app.set_file_preview(FilePreview::new(
            "a".into(),
            "body".into(),
            PreviewKind::Text,
        ));
        app.store_rendered_file_preview(80, vec![Line::from("body")]);
        let revision = app.file_preview_revision;

        app.set_file_preview(FilePreview::new(
            "a".into(),
            "changed".into(),
            PreviewKind::Text,
        ));

        assert_ne!(app.file_preview_revision, revision);
        assert!(app.rendered_file_preview.is_none());
    }
}
