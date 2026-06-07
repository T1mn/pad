use super::super::db::{ensure_schema_at, open_db, to_io_error};
use super::super::{ThreadMeta, ThreadMetaKey};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;

pub(in crate::thread_meta) fn load_thread_meta_batch_at(
    db_path: &Path,
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
        normalize_meta(meta);
    }

    Ok(records)
}

pub(in crate::thread_meta) fn load_deleted_thread_meta_at(
    db_path: &Path,
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

fn normalize_meta(meta: &mut ThreadMeta) {
    dedup_tags(&mut meta.tags);
    meta.title_override = meta.title_override.as_ref().and_then(|s| clean_text(s));
    meta.generated_title = meta.generated_title.as_ref().and_then(|s| clean_text(s));
    meta.note = meta.note.as_ref().and_then(|s| clean_text(s));
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
