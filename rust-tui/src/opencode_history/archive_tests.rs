use super::set_archived_at;

#[test]
fn archive_matches_upstream_semantics_without_reordering_session() {
    let path = crate::test_support::temp_path("pad-opencode", "archive.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE session (
                id TEXT PRIMARY KEY, time_updated INTEGER NOT NULL, time_archived INTEGER
            );
            INSERT INTO session (id, time_updated) VALUES ('session-1', 42);",
        )
        .unwrap();

    set_archived_at(&path, "session-1", true).unwrap();
    let archived: (i64, Option<i64>) = connection
        .query_row(
            "SELECT time_updated, time_archived FROM session WHERE id='session-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(archived.0, 42);
    assert!(archived.1.is_some());

    set_archived_at(&path, "session-1", false).unwrap();
    let restored: (i64, Option<i64>) = connection
        .query_row(
            "SELECT time_updated, time_archived FROM session WHERE id='session-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(restored, (42, None));
    let _ = std::fs::remove_file(path);
}
