use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub(super) fn temp_codex_home(name: &str) -> PathBuf {
    let path = crate::test_support::temp_path("pad-codex-provider-sync", name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create temp codex home");
    path
}

pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    crate::test_support::with_temp_home("pad-codex-provider-sync-home", name, f)
}

pub(super) fn write_rollout(path: &Path, thread_id: &str, provider: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create rollout dir");
    }
    let first_line = serde_json::json!({
        "timestamp": "2026-04-10T00:00:00Z",
        "type": "session_meta",
        "payload": {
            "id": thread_id,
            "model_provider": provider,
            "cwd": "/repo"
        }
    });
    std::fs::write(
        path,
        format!(
            "{}\n{{\"type\":\"event_msg\",\"payload\":{{}}}}\n",
            first_line
        ),
    )
    .expect("write rollout");
}

pub(super) fn write_state_db(path: &Path) {
    let connection = Connection::open(path).expect("open db");
    connection
        .execute_batch(
            "CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    model_provider TEXT NOT NULL,
                    archived INTEGER NOT NULL DEFAULT 0
                );
                INSERT INTO threads (id, model_provider, archived) VALUES
                    ('thread-a', 'old', 0),
                    ('thread-b', 'older', 1);",
        )
        .expect("seed db");
}

pub(super) fn rollout_text(home: &Path, relative: &str) -> String {
    std::fs::read_to_string(home.join(relative)).expect("read rollout")
}
