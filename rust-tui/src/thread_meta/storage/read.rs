mod normalize;
mod row;
mod tags;

use super::super::db::{ensure_schema_at, open_db, to_io_error};
use super::super::{ThreadMeta, ThreadMetaKey};
use normalize::normalize_meta;
use row::{thread_meta_from_row, THREAD_META_COLUMNS};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;
use tags::{hydrate_deleted_tags, load_tags_into_records};

pub(in crate::thread_meta) fn load_thread_meta_batch_at(
    db_path: &Path,
    keys: &[ThreadMetaKey],
) -> io::Result<HashMap<ThreadMetaKey, ThreadMeta>> {
    if keys.is_empty() {
        return Ok(HashMap::new());
    }
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let wanted = keys
        .iter()
        .map(|key| (key.agent_type.as_str(), key.thread_id.as_str()))
        .collect::<HashSet<_>>();
    let mut records = load_wanted_meta_records(&connection, &wanted)?;
    load_tags_into_records(&connection, &wanted, &mut records)?;

    for meta in records.values_mut() {
        normalize_meta(meta);
    }

    Ok(records)
}

pub(in crate::thread_meta) fn load_deleted_thread_meta_at(
    db_path: &Path,
) -> io::Result<Vec<(ThreadMetaKey, ThreadMeta)>> {
    ensure_schema_at(db_path)?;
    let connection = open_db(db_path)?;
    let sql = format!(
        "SELECT {THREAD_META_COLUMNS}
         FROM thread_meta
         WHERE deleted = 1
         ORDER BY deleted_at DESC, updated_at DESC"
    );
    let mut statement = connection.prepare(&sql).map_err(to_io_error)?;
    let rows = statement
        .query_map([], thread_meta_from_row)
        .map_err(to_io_error)?;
    let mut deleted = rows.collect::<Result<Vec<_>, _>>().map_err(to_io_error)?;
    hydrate_deleted_tags(&connection, &mut deleted)?;
    for (_, meta) in &mut deleted {
        normalize_meta(meta);
    }
    Ok(deleted)
}

pub(in crate::thread_meta) fn deleted_thread_count_at(db_path: &Path) -> io::Result<usize> {
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

fn load_wanted_meta_records(
    connection: &rusqlite::Connection,
    wanted: &HashSet<(&str, &str)>,
) -> io::Result<HashMap<ThreadMetaKey, ThreadMeta>> {
    let sql = format!("SELECT {THREAD_META_COLUMNS} FROM thread_meta");
    let mut statement = connection.prepare(&sql).map_err(to_io_error)?;
    let rows = statement
        .query_map([], thread_meta_from_row)
        .map_err(to_io_error)?;
    let mut records = HashMap::new();

    for row in rows {
        let (key, meta) = row.map_err(to_io_error)?;
        if wanted.contains(&(key.agent_type.as_str(), key.thread_id.as_str())) {
            records.insert(key, meta);
        }
    }

    Ok(records)
}
