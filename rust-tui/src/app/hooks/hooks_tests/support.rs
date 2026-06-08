use crate::hook::{HookEvent, HookTmuxInfo};
use rusqlite::Connection;
use std::path::Path;

pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    crate::test_support::with_temp_home("pad-hooks", name, f)
}

pub(super) fn create_codex_threads_db(path: &Path) {
    let connection = Connection::open(path).expect("open sqlite");
    connection
        .execute_batch(
            "CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    cwd TEXT NOT NULL,
                    updated_at INTEGER NOT NULL,
                    rollout_path TEXT NOT NULL,
                    title TEXT,
                    first_user_message TEXT,
                    source TEXT,
                    archived INTEGER NOT NULL DEFAULT 0,
                    archived_at INTEGER
                );",
        )
        .expect("create threads table");
}

pub(super) fn insert_codex_thread(path: &Path, thread_id: &str, title: &str) {
    let connection = Connection::open(path).expect("open sqlite");
    connection
        .execute(
            "INSERT INTO threads (
                    id, cwd, updated_at, rollout_path, title, first_user_message, source, archived, archived_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, NULL)",
            (
                thread_id,
                "/tmp/demo",
                1_i64,
                format!("/tmp/{thread_id}.jsonl"),
                title,
                "old first prompt",
                "cli",
            ),
        )
        .expect("insert thread");
}

pub(super) fn stop_event(pane_id: &str) -> HookEvent {
    HookEvent {
        event: "stop".into(),
        turn_id: Some("turn-1".into()),
        session_id: Some("session-1".into()),
        transcript_path: None,
        cwd: None,
        prompt: None,
        last_assistant_message: Some("done".into()),
        timestamp: None,
        tmux: HookTmuxInfo {
            pane_id: Some(pane_id.into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: Some("/tmp/demo".into()),
        },
    }
}

pub(super) fn submit_event(pane_id: Option<&str>) -> HookEvent {
    HookEvent {
        event: "user_prompt_submit".into(),
        turn_id: Some("turn-1".into()),
        session_id: Some("session-1".into()),
        transcript_path: None,
        cwd: Some("/tmp/demo".into()),
        prompt: Some("ship it".into()),
        last_assistant_message: None,
        timestamp: None,
        tmux: HookTmuxInfo {
            pane_id: pane_id.map(str::to_string),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: Some("/tmp/demo".into()),
        },
    }
}
