use super::helpers::{
    rollout_text, temp_codex_home, with_temp_home, write_rollout, write_state_db,
};
use super::{sync_to_provider, sync_to_provider_at};
use rusqlite::Connection;

#[test]
fn sync_updates_rollout_files_and_sqlite_provider() {
    let codex_home = temp_codex_home("success");
    write_rollout(
        &codex_home.join("sessions/2026/04/10/rollout-a.jsonl"),
        "thread-a",
        "old",
    );
    write_rollout(
        &codex_home.join("archived_sessions/2026/04/09/rollout-b.jsonl"),
        "thread-b",
        "older",
    );
    write_state_db(&codex_home.join("state_5.sqlite"));

    let result = sync_to_provider_at(&codex_home, "openai").expect("sync provider");

    assert_eq!(
        result,
        super::ProviderSyncResult {
            updated_rollout_files: 2,
            updated_sqlite_rows: 2,
        }
    );

    let session_rollout = rollout_text(&codex_home, "sessions/2026/04/10/rollout-a.jsonl");
    assert!(session_rollout.contains("\"model_provider\":\"openai\""));
    assert!(session_rollout.contains("\"type\":\"event_msg\""));

    let archived_rollout =
        rollout_text(&codex_home, "archived_sessions/2026/04/09/rollout-b.jsonl");
    assert!(archived_rollout.contains("\"model_provider\":\"openai\""));

    let connection = Connection::open(codex_home.join("state_5.sqlite")).expect("open db");
    let providers = connection
        .prepare("SELECT model_provider FROM threads ORDER BY id")
        .expect("prepare query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query providers")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect providers");
    assert_eq!(providers, vec!["openai".to_string(), "openai".to_string()]);

    let _ = std::fs::remove_dir_all(&codex_home);
}

#[test]
fn sync_skips_when_state_db_is_missing() {
    let codex_home = temp_codex_home("no-db");
    write_rollout(
        &codex_home.join("sessions/2026/04/10/rollout-a.jsonl"),
        "thread-a",
        "old",
    );

    let result = sync_to_provider_at(&codex_home, "openai").expect("sync provider");

    assert_eq!(result.updated_rollout_files, 1);
    assert_eq!(result.updated_sqlite_rows, 0);

    let rollout = rollout_text(&codex_home, "sessions/2026/04/10/rollout-a.jsonl");
    assert!(rollout.contains("\"model_provider\":\"openai\""));

    let _ = std::fs::remove_dir_all(&codex_home);
}

#[test]
fn sync_to_provider_uses_pad_private_codex_home() {
    with_temp_home("private-home", |_home| {
        let canonical_home = crate::paths::canonical_codex_home_dir();
        let pad_home = crate::paths::pad_codex_home_dir();

        write_rollout(
            &canonical_home.join("sessions/2026/04/10/rollout-canonical.jsonl"),
            "thread-a",
            "canonical-old",
        );
        write_state_db(&canonical_home.join("state_5.sqlite"));

        write_rollout(
            &pad_home.join("sessions/2026/04/10/rollout-pad.jsonl"),
            "thread-a",
            "pad-old",
        );
        write_state_db(&pad_home.join("state_5.sqlite"));

        let result = sync_to_provider("pad-provider").expect("sync provider");

        assert_eq!(
            result,
            super::ProviderSyncResult {
                updated_rollout_files: 1,
                updated_sqlite_rows: 2,
            }
        );
        let pad_rollout = rollout_text(&pad_home, "sessions/2026/04/10/rollout-pad.jsonl");
        assert!(pad_rollout.contains("\"model_provider\":\"pad-provider\""));
        let canonical_rollout = rollout_text(
            &canonical_home,
            "sessions/2026/04/10/rollout-canonical.jsonl",
        );
        assert!(canonical_rollout.contains("\"model_provider\":\"canonical-old\""));

        let canonical_connection =
            Connection::open(canonical_home.join("state_5.sqlite")).expect("open canonical db");
        let canonical_provider = canonical_connection
            .query_row(
                "SELECT model_provider FROM threads WHERE id = 'thread-a'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("query canonical provider");
        assert_eq!(canonical_provider, "old");
    });
}
