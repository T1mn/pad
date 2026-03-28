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
    Ok(load_thread_meta_batch(&[key.clone()])?.remove(&key))
}

pub fn upsert_thread_meta(
    agent_type: &str,
    thread_id: &str,
    title_override: Option<&str>,
    note: Option<&str>,
    pinned: bool,
) -> io::Result<()> {
    let path = db_path();
    ensure_schema_at(&path)?;
    let connection = open_db(&path)?;
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
                "SELECT agent_type, thread_id, title_override, note, pinned, updated_at
                 FROM thread_meta",
            )
            .map_err(to_io_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    ThreadMetaKey::new(row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    ThreadMeta {
                        title_override: row.get::<_, Option<String>>(2)?,
                        note: row.get::<_, Option<String>>(3)?,
                        pinned: row.get::<_, i64>(4)? != 0,
                        tags: Vec::new(),
                        updated_at: row.get::<_, i64>(5)?,
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
        if let Some(note) = meta.note.as_ref().and_then(|s| clean_text(s)) {
            meta.note = Some(note);
        } else {
            meta.note = None;
        }
    }

    Ok(records)
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

fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}
