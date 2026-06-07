use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn temp_codex_home(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "pad-codex-provider-sync-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create temp codex home");
    path
}

fn temp_home(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("pad-codex-provider-sync-home-{name}-{stamp}"))
}

pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock provider sync tests");
    let home = temp_home(name);
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let result = f(&home);

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);

    result
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
