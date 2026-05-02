use super::*;

#[test]
fn recent_history_message_shows_only_latest_three_turns() {
    let turns = vec![
        PreviewTurn {
            question: "latest".into(),
            answer: Some("answer latest".into()),
        },
        PreviewTurn {
            question: "second".into(),
            answer: Some("answer second".into()),
        },
        PreviewTurn {
            question: "third".into(),
            answer: Some("answer third".into()),
        },
    ];

    let body = format_recent_history_message(crate::i18n::Locale::En, "CODEX • demo", &turns);
    assert!(body.contains("Recent 3 turns"));
    assert!(body.contains("CODEX • demo"));
    assert!(body.contains("latest"));
    assert!(body.contains("answer latest"));
    assert!(body.contains("third"));
    assert!(body.contains("answer third"));
}
#[test]
fn recent_history_turns_prefers_latest_three_cached_turns() {
    let panel = sample_panel_with_turns(vec![
        PreviewTurn {
            question: "q1".into(),
            answer: Some("a1".into()),
        },
        PreviewTurn {
            question: "q2".into(),
            answer: Some("a2".into()),
        },
        PreviewTurn {
            question: "q3".into(),
            answer: Some("a3".into()),
        },
        PreviewTurn {
            question: "q4".into(),
            answer: Some("a4".into()),
        },
    ]);

    let turns = recent_history_turns(&panel, crate::i18n::Locale::En);
    assert_eq!(turns.len(), 3);
    assert_eq!(turns[0].question, "q1");
    assert_eq!(turns[1].question, "q2");
    assert_eq!(turns[2].question, "q3");
}
#[test]
fn recent_history_turns_reads_codex_rollout_from_db_by_workdir() {
    with_temp_home("recent-history-codex-db", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).unwrap();

        let rollout_path = codex_dir.join("session-db-1.jsonl");
        fs::write(
            &rollout_path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"real question\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"real answer\"}]}}\n"
            ),
        )
        .unwrap();

        let db_path = codex_dir.join("state_5.sqlite");
        let connection = rusqlite::Connection::open(&db_path).unwrap();
        connection
            .execute_batch(
                "
                CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    cwd TEXT NOT NULL,
                    updated_at INTEGER NOT NULL,
                    rollout_path TEXT NOT NULL,
                    title TEXT,
                    first_user_message TEXT,
                    source TEXT,
                    archived INTEGER NOT NULL DEFAULT 0,
                    archived_at INTEGER
                );
                ",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO threads (
                    id, cwd, updated_at, rollout_path, title, first_user_message, source, archived, archived_at
                ) VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, 0, NULL)",
                rusqlite::params![
                    "session-db-1",
                    "/tmp/rust-tui",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("time")
                        .as_secs() as i64,
                    rollout_path.to_string_lossy().to_string(),
                ],
            )
            .unwrap();

        let mut panel = sample_panel_with_turns(Vec::new());
        panel.working_dir = "/tmp/rust-tui".into();
        panel.agent_session_id = None;

        let turns = recent_history_turns(&panel, crate::i18n::Locale::En);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "real question");
        assert_eq!(turns[0].answer.as_deref(), Some("real answer"));
    });
}
#[test]
fn restart_shell_command_drops_telegram_bot_and_keeps_debug_flag() {
    let command = build_pad_restart_shell_command(
        Path::new("/tmp/pad"),
        &["/tmp/pad".into(), "telegram-bot".into(), "--debug".into()],
        None,
    );

    assert!(command.contains("cargo build"));
    assert!(command.contains("exec '/tmp/pad' '--debug'"));
    assert!(!command.contains("telegram-bot"));
}
#[test]
fn restart_shell_command_uses_release_profile_when_binary_is_release() {
    let command = build_pad_restart_shell_command(
        Path::new("/tmp/target/release/pad"),
        &["/tmp/target/release/pad".into()],
        Some("/tmp/custom-target"),
    );

    assert!(command.contains("export CARGO_TARGET_DIR='/tmp/custom-target'"));
    assert!(command.contains("cargo build --release"));
}
#[test]
fn restart_target_prefers_current_tmux_pane() {
    let target = select_pad_restart_target(
        Some("%99"),
        "pad",
        &[SessionPaneInfo {
            pane_id: "%1".into(),
            pid: Some(10),
            command: "pad".into(),
        }],
        Some(10),
        "pad",
    );

    assert_eq!(target, PadRestartTarget::RespawnPane("%99".into()));
}
#[test]
fn restart_target_prefers_pid_match_before_command_match() {
    let target = select_pad_restart_target(
        None,
        "pad",
        &[
            SessionPaneInfo {
                pane_id: "%1".into(),
                pid: Some(99),
                command: "bash".into(),
            },
            SessionPaneInfo {
                pane_id: "%2".into(),
                pid: Some(10),
                command: "pad".into(),
            },
        ],
        Some(99),
        "pad",
    );

    assert_eq!(target, PadRestartTarget::RespawnPane("%1".into()));
}
#[test]
fn restart_target_creates_new_session_when_no_existing_pane_is_available() {
    let target = select_pad_restart_target(None, "pad", &[], None, "pad");
    assert_eq!(target, PadRestartTarget::NewDetachedSession("pad".into()));
}
