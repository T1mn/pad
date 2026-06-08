use super::db::{ensure_schema_at, open_db};
use super::storage::{
    deleted_thread_count_at, load_deleted_thread_meta_at, load_thread_meta_batch_at,
    set_thread_deleted_at, upsert_generated_title_at, upsert_thread_meta_at,
};
use super::ThreadMetaKey;
use rusqlite::params;

fn temp_db_path(name: &str) -> std::path::PathBuf {
    crate::test_support::temp_path("pad-thread-meta", name)
}

#[test]
fn ensure_schema_adds_generated_title_columns_to_existing_db() {
    let db_path = temp_db_path("migration");
    let _ = std::fs::remove_file(&db_path);
    let connection = open_db(&db_path).expect("open temp db");
    connection
        .execute_batch(
            "CREATE TABLE thread_meta (
                    agent_type TEXT NOT NULL,
                    thread_id TEXT NOT NULL,
                    title_override TEXT,
                    note TEXT,
                    pinned INTEGER NOT NULL DEFAULT 0,
                    updated_at INTEGER NOT NULL,
                    PRIMARY KEY(agent_type, thread_id)
                );",
        )
        .expect("seed old schema");

    ensure_schema_at(&db_path).expect("migrate schema");

    let mut statement = connection
        .prepare("PRAGMA table_info(thread_meta)")
        .expect("prepare pragma");
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("query columns")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect columns");

    assert!(columns.iter().any(|name| name == "generated_title"));
    assert!(columns.iter().any(|name| name == "generated_turn_count"));
    assert!(columns.iter().any(|name| name == "generated_updated_at"));

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn generated_title_updates_do_not_clobber_manual_override() {
    let db_path = temp_db_path("generated-title");
    let _ = std::fs::remove_file(&db_path);

    upsert_thread_meta_at(
        &db_path,
        "codex",
        "sid-1",
        Some("Manual"),
        Some("note"),
        true,
    )
    .expect("save manual meta");
    upsert_generated_title_at(&db_path, "codex", "sid-1", "Generated", 9)
        .expect("save generated title");

    let key = ThreadMetaKey::new("codex", "sid-1");
    let meta = load_thread_meta_batch_at(&db_path, std::slice::from_ref(&key))
        .expect("load meta")
        .remove(&key)
        .expect("meta row");

    assert_eq!(meta.title_override.as_deref(), Some("Manual"));
    assert_eq!(meta.generated_title.as_deref(), Some("Generated"));
    assert_eq!(meta.generated_turn_count, Some(9));
    assert_eq!(meta.note.as_deref(), Some("note"));
    assert!(meta.pinned);

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn load_thread_meta_reads_generated_fields() {
    let db_path = temp_db_path("load-generated");
    let _ = std::fs::remove_file(&db_path);
    ensure_schema_at(&db_path).expect("ensure schema");
    let connection = open_db(&db_path).expect("open temp db");
    connection
        .execute(
            "INSERT INTO thread_meta (
                    agent_type, thread_id, title_override, generated_title,
                    generated_turn_count, generated_updated_at, note, pinned, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "codex",
                "sid-1",
                Option::<String>::None,
                Some("Generated title".to_string()),
                15_i64,
                123_i64,
                Option::<String>::None,
                0_i64,
                123_i64,
            ],
        )
        .expect("insert row");

    let key = ThreadMetaKey::new("codex", "sid-1");
    let meta = load_thread_meta_batch_at(&db_path, std::slice::from_ref(&key))
        .expect("load meta")
        .remove(&key)
        .expect("meta row");

    assert_eq!(meta.generated_title.as_deref(), Some("Generated title"));
    assert_eq!(meta.generated_turn_count, Some(15));
    assert_eq!(meta.generated_updated_at, Some(123));

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn set_thread_deleted_marks_and_clears_deleted_state() {
    let db_path = temp_db_path("deleted-toggle");
    let _ = std::fs::remove_file(&db_path);

    upsert_thread_meta_at(&db_path, "codex", "sid-1", Some("Manual"), None, false)
        .expect("seed meta");
    set_thread_deleted_at(&db_path, "codex", "sid-1", true).expect("mark deleted");

    let key = ThreadMetaKey::new("codex", "sid-1");
    let meta = load_thread_meta_batch_at(&db_path, std::slice::from_ref(&key))
        .expect("load meta")
        .remove(&key)
        .expect("meta row");
    assert!(meta.deleted);
    assert!(meta.deleted_at.is_some());
    assert_eq!(deleted_thread_count_at(&db_path).expect("deleted count"), 1);

    set_thread_deleted_at(&db_path, "codex", "sid-1", false).expect("clear deleted");
    let meta = load_thread_meta_batch_at(&db_path, std::slice::from_ref(&key))
        .expect("reload meta")
        .remove(&key)
        .expect("meta row");
    assert!(!meta.deleted);
    assert_eq!(meta.deleted_at, None);
    assert_eq!(deleted_thread_count_at(&db_path).expect("deleted count"), 0);

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn load_deleted_thread_meta_returns_only_deleted_rows() {
    let db_path = temp_db_path("deleted-list");
    let _ = std::fs::remove_file(&db_path);

    upsert_thread_meta_at(&db_path, "codex", "sid-1", Some("Keep"), None, false)
        .expect("seed keep");
    upsert_thread_meta_at(&db_path, "codex", "sid-2", Some("Trash"), None, false)
        .expect("seed trash");
    set_thread_deleted_at(&db_path, "codex", "sid-2", true).expect("mark deleted");

    let deleted = load_deleted_thread_meta_at(&db_path).expect("load deleted");
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].0.thread_id, "sid-2");
    assert!(deleted[0].1.deleted);

    let _ = std::fs::remove_file(&db_path);
}
