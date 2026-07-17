use super::codex_transcript_path_for_session_id;

#[test]
fn codex_db_canonical_path_resolves_compressed_sibling() {
    crate::test_support::with_temp_home("pad-codex-target", "compressed", |home| {
        let codex_home = home.join(".codex");
        std::fs::create_dir_all(&codex_home).unwrap();
        let canonical = codex_home.join("sessions/rollout-session-zst.jsonl");
        let compressed = canonical.with_extension("jsonl.zst");
        std::fs::create_dir_all(compressed.parent().unwrap()).unwrap();
        std::fs::write(&compressed, b"fixture").unwrap();

        let connection = rusqlite::Connection::open(codex_home.join("state_5.sqlite")).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE threads (
                    id TEXT PRIMARY KEY, cwd TEXT NOT NULL, updated_at INTEGER NOT NULL,
                    rollout_path TEXT NOT NULL, title TEXT, first_user_message TEXT,
                    source TEXT, archived INTEGER NOT NULL DEFAULT 0, archived_at INTEGER
                );",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO threads (id,cwd,updated_at,rollout_path,archived)
                 VALUES (?1,?2,?3,?4,0)",
                rusqlite::params![
                    "session-zst",
                    "/tmp/project",
                    1_i64,
                    canonical.to_string_lossy().to_string()
                ],
            )
            .unwrap();

        assert_eq!(
            codex_transcript_path_for_session_id("session-zst"),
            Some(compressed)
        );
    });
}
