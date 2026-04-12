use rusqlite::{params, Connection, OpenFlags};
use std::collections::{HashMap, HashSet};
use std::io;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ThreadMetaKey {
    pub agent_type: String,
    pub thread_id: String,
}

impl ThreadMetaKey {
    pub fn new(agent_type: impl Into<String>, thread_id: impl Into<String>) -> Self {
        Self {
            agent_type: agent_type.into(),
            thread_id: thread_id.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadMeta {
    pub title_override: Option<String>,
    pub generated_title: Option<String>,
    pub generated_turn_count: Option<usize>,
    pub generated_updated_at: Option<i64>,
    pub deleted: bool,
    pub deleted_at: Option<i64>,
    pub note: Option<String>,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub updated_at: i64,
}

pub fn ensure_db() -> io::Result<()> {
    ensure_schema_at(&db_path())
}

pub fn load_thread_meta_batch(
    keys: &[ThreadMetaKey],
) -> io::Result<HashMap<ThreadMetaKey, ThreadMeta>> {
    load_thread_meta_batch_at(&db_path(), keys)
}

pub fn load_thread_meta(agent_type: &str, thread_id: &str) -> io::Result<Option<ThreadMeta>> {
    let key = ThreadMetaKey::new(agent_type, thread_id);
    Ok(load_thread_meta_batch(std::slice::from_ref(&key))?.remove(&key))
}

pub fn upsert_thread_meta(
    agent_type: &str,
    thread_id: &str,
    title_override: Option<&str>,
    note: Option<&str>,
    pinned: bool,
) -> io::Result<()> {
    upsert_thread_meta_at(
        &db_path(),
        agent_type,
        thread_id,
        title_override,
        note,
        pinned,
    )
}

pub fn upsert_generated_title(
    agent_type: &str,
    thread_id: &str,
    generated_title: &str,
    generated_turn_count: usize,
) -> io::Result<()> {
    upsert_generated_title_at(
        &db_path(),
        agent_type,
        thread_id,
        generated_title,
        generated_turn_count,
    )
}

pub fn set_thread_deleted(agent_type: &str, thread_id: &str, deleted: bool) -> io::Result<()> {
    set_thread_deleted_at(&db_path(), agent_type, thread_id, deleted)
}

pub fn deleted_thread_count() -> io::Result<usize> {
    deleted_thread_count_at(&db_path())
}

pub fn load_deleted_thread_meta() -> io::Result<Vec<(ThreadMetaKey, ThreadMeta)>> {
    load_deleted_thread_meta_at(&db_path())
}

fn upsert_thread_meta_at(
    db_path: &std::path::Path,
    agent_type: &str,
    thread_id: &str,
    title_override: Option<&str>,
    note: Option<&str>,
    pinned: bool,
) -> io::Result<()> {
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let now = crate::app::unix_now_ts();
    connection
        .execute(
            "INSERT INTO thread_meta (
                agent_type, thread_id, title_override, note, pinned, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(agent_type, thread_id) DO UPDATE SET
                title_override = excluded.title_override,
                note = excluded.note,
                pinned = excluded.pinned,
                updated_at = excluded.updated_at",
            params![
                agent_type,
                thread_id,
                normalize_text(title_override),
                normalize_text(note),
                if pinned { 1_i64 } else { 0_i64 },
                now,
            ],
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn upsert_generated_title_at(
    db_path: &std::path::Path,
    agent_type: &str,
    thread_id: &str,
    generated_title: &str,
    generated_turn_count: usize,
) -> io::Result<()> {
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let now = crate::app::unix_now_ts();
    connection
        .execute(
            "INSERT INTO thread_meta (
                agent_type,
                thread_id,
                title_override,
                generated_title,
                generated_turn_count,
                generated_updated_at,
                note,
                pinned,
                updated_at
             ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, NULL, 0, ?5)
             ON CONFLICT(agent_type, thread_id) DO UPDATE SET
                generated_title = excluded.generated_title,
                generated_turn_count = excluded.generated_turn_count,
                generated_updated_at = excluded.generated_updated_at,
                updated_at = excluded.updated_at",
            params![
                agent_type,
                thread_id,
                normalize_text(Some(generated_title)),
                generated_turn_count as i64,
                now,
            ],
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn set_thread_deleted_at(
    db_path: &std::path::Path,
    agent_type: &str,
    thread_id: &str,
    deleted: bool,
) -> io::Result<()> {
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let now = crate::app::unix_now_ts();
    connection
        .execute(
            "INSERT INTO thread_meta (
                agent_type,
                thread_id,
                title_override,
                generated_title,
                generated_turn_count,
                generated_updated_at,
                deleted,
                deleted_at,
                note,
                pinned,
                updated_at
             ) VALUES (?1, ?2, NULL, NULL, NULL, NULL, ?3, ?4, NULL, 0, ?4)
             ON CONFLICT(agent_type, thread_id) DO UPDATE SET
                deleted = excluded.deleted,
                deleted_at = CASE WHEN excluded.deleted = 1 THEN excluded.deleted_at ELSE NULL END,
                updated_at = excluded.updated_at",
            params![
                agent_type,
                thread_id,
                if deleted { 1_i64 } else { 0_i64 },
                now,
            ],
        )
        .map_err(to_io_error)?;
    Ok(())
}

pub fn replace_thread_tags(agent_type: &str, thread_id: &str, tags: &[String]) -> io::Result<()> {
    let path = db_path();
    ensure_schema_at(&path)?;
    let mut connection = open_db(&path)?;
    let tx = connection.transaction().map_err(to_io_error)?;
    tx.execute(
        "DELETE FROM thread_tags WHERE agent_type = ?1 AND thread_id = ?2",
        params![agent_type, thread_id],
    )
    .map_err(to_io_error)?;

    let mut seen = HashSet::new();
    let now = crate::app::unix_now_ts();
    for tag in tags {
        let normalized = tag.trim();
        if normalized.is_empty() {
            continue;
        }
        if !seen.insert(normalized.to_string()) {
            continue;
        }
        tx.execute(
            "INSERT INTO thread_tags (agent_type, thread_id, tag, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![agent_type, thread_id, normalized, now],
        )
        .map_err(to_io_error)?;
    }

    tx.commit().map_err(to_io_error)?;
    Ok(())
}

fn load_thread_meta_batch_at(
    db_path: &std::path::Path,
    keys: &[ThreadMetaKey],
) -> io::Result<HashMap<ThreadMetaKey, ThreadMeta>> {
    if keys.is_empty() {
        return Ok(HashMap::new());
    }
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let wanted: HashSet<ThreadMetaKey> = keys.iter().cloned().collect();
    let mut records: HashMap<ThreadMetaKey, ThreadMeta> = HashMap::new();

    {
        let mut statement = connection
            .prepare(
                "SELECT agent_type, thread_id, title_override, generated_title,
                        generated_turn_count, generated_updated_at, deleted, deleted_at,
                        note, pinned, updated_at
                 FROM thread_meta",
            )
            .map_err(to_io_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    ThreadMetaKey::new(row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    ThreadMeta {
                        title_override: row.get::<_, Option<String>>(2)?,
                        generated_title: row.get::<_, Option<String>>(3)?,
                        generated_turn_count: row
                            .get::<_, Option<i64>>(4)?
                            .filter(|count| *count > 0)
                            .map(|count| count as usize),
                        generated_updated_at: row.get::<_, Option<i64>>(5)?,
                        deleted: row.get::<_, i64>(6)? != 0,
                        deleted_at: row.get::<_, Option<i64>>(7)?,
                        note: row.get::<_, Option<String>>(8)?,
                        pinned: row.get::<_, i64>(9)? != 0,
                        tags: Vec::new(),
                        updated_at: row.get::<_, i64>(10)?,
                    },
                ))
            })
            .map_err(to_io_error)?;

        for row in rows {
            let (key, meta) = row.map_err(to_io_error)?;
            if wanted.contains(&key) {
                records.insert(key, meta);
            }
        }
    }

    {
        let mut statement = connection
            .prepare(
                "SELECT agent_type, thread_id, tag, created_at
                 FROM thread_tags
                 ORDER BY created_at ASC",
            )
            .map_err(to_io_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    ThreadMetaKey::new(row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(to_io_error)?;

        for row in rows {
            let (key, tag) = row.map_err(to_io_error)?;
            if !wanted.contains(&key) {
                continue;
            }
            records.entry(key).or_default().tags.push(tag);
        }
    }

    for meta in records.values_mut() {
        dedup_tags(&mut meta.tags);
        if let Some(title) = meta.title_override.as_ref().and_then(|s| clean_text(s)) {
            meta.title_override = Some(title);
        } else {
            meta.title_override = None;
        }
        if let Some(title) = meta.generated_title.as_ref().and_then(|s| clean_text(s)) {
            meta.generated_title = Some(title);
        } else {
            meta.generated_title = None;
        }
        if let Some(note) = meta.note.as_ref().and_then(|s| clean_text(s)) {
            meta.note = Some(note);
        } else {
            meta.note = None;
        }
    }

    Ok(records)
}

fn load_deleted_thread_meta_at(
    db_path: &std::path::Path,
) -> io::Result<Vec<(ThreadMetaKey, ThreadMeta)>> {
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT agent_type, thread_id, title_override, generated_title,
                    generated_turn_count, generated_updated_at, deleted, deleted_at,
                    note, pinned, updated_at
             FROM thread_meta
             WHERE deleted = 1
             ORDER BY deleted_at DESC, updated_at DESC",
        )
        .map_err(to_io_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                ThreadMetaKey::new(row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                ThreadMeta {
                    title_override: row.get::<_, Option<String>>(2)?,
                    generated_title: row.get::<_, Option<String>>(3)?,
                    generated_turn_count: row
                        .get::<_, Option<i64>>(4)?
                        .filter(|count| *count > 0)
                        .map(|count| count as usize),
                    generated_updated_at: row.get::<_, Option<i64>>(5)?,
                    deleted: row.get::<_, i64>(6)? != 0,
                    deleted_at: row.get::<_, Option<i64>>(7)?,
                    note: row.get::<_, Option<String>>(8)?,
                    pinned: row.get::<_, i64>(9)? != 0,
                    tags: Vec::new(),
                    updated_at: row.get::<_, i64>(10)?,
                },
            ))
        })
        .map_err(to_io_error)?;
    let mut deleted = rows.collect::<Result<Vec<_>, _>>().map_err(to_io_error)?;

    let keys = deleted
        .iter()
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let tags = load_thread_meta_batch_at(db_path, &keys)?;
    for (key, meta) in &mut deleted {
        if let Some(tag_meta) = tags.get(key) {
            meta.tags = tag_meta.tags.clone();
        }
    }
    Ok(deleted)
}

fn deleted_thread_count_at(db_path: &std::path::Path) -> io::Result<usize> {
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let count = connection
        .query_row(
            "SELECT COUNT(*) FROM thread_meta WHERE deleted = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(to_io_error)?;
    Ok(count.max(0) as usize)
}

fn ensure_schema_at(db_path: &std::path::Path) -> io::Result<()> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let connection = open_db(db_path)?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS thread_meta (
                agent_type TEXT NOT NULL,
                thread_id TEXT NOT NULL,
                title_override TEXT,
                generated_title TEXT,
                generated_turn_count INTEGER,
                generated_updated_at INTEGER,
                deleted INTEGER NOT NULL DEFAULT 0,
                deleted_at INTEGER,
                note TEXT,
                pinned INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY(agent_type, thread_id)
            );
            CREATE TABLE IF NOT EXISTS thread_tags (
                agent_type TEXT NOT NULL,
                thread_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY(agent_type, thread_id, tag)
            );
            CREATE INDEX IF NOT EXISTS idx_thread_meta_updated_at
                ON thread_meta(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_thread_tags_thread
                ON thread_tags(agent_type, thread_id);",
        )
        .map_err(to_io_error)?;
    ensure_column(&connection, "generated_title", "TEXT")?;
    ensure_column(&connection, "generated_turn_count", "INTEGER")?;
    ensure_column(&connection, "generated_updated_at", "INTEGER")?;
    ensure_column(&connection, "deleted", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(&connection, "deleted_at", "INTEGER")?;
    Ok(())
}

fn open_db(db_path: &std::path::Path) -> io::Result<Connection> {
    Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)
}

fn db_path() -> std::path::PathBuf {
    crate::paths::pad_db_path()
}

fn normalize_text(value: Option<&str>) -> Option<String> {
    value.and_then(clean_text)
}

fn clean_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn dedup_tags(tags: &mut Vec<String>) {
    let mut seen = HashSet::new();
    tags.retain(|tag| seen.insert(tag.to_lowercase()));
}

fn ensure_column(connection: &Connection, column: &str, definition: &str) -> io::Result<()> {
    let mut statement = connection
        .prepare("PRAGMA table_info(thread_meta)")
        .map_err(to_io_error)?;
    let existing = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(to_io_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_io_error)?;

    if existing.iter().any(|name| name == column) {
        return Ok(());
    }

    connection
        .execute(
            &format!("ALTER TABLE thread_meta ADD COLUMN {column} {definition}"),
            [],
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}

#[cfg(test)]
mod tests {
    use super::{
        deleted_thread_count_at, ensure_schema_at, load_deleted_thread_meta_at,
        load_thread_meta_batch_at, open_db, set_thread_deleted_at, upsert_generated_title_at,
        upsert_thread_meta_at, ThreadMetaKey,
    };
    use rusqlite::params;

    fn temp_db_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "pad-thread-meta-{name}-{}.sqlite",
            std::process::id()
        ))
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
}
