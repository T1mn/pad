mod normalize {
    use super::super::super::ThreadMeta;
    use std::collections::HashSet;

    pub(super) fn normalize_meta(meta: &mut ThreadMeta) {
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
}
mod row {
    use super::super::super::{ThreadMeta, ThreadMetaKey};

    pub(super) const THREAD_META_COLUMNS: &str = concat!(
        "agent_type, thread_id, title_override, generated_title, ",
        "generated_turn_count, generated_updated_at, deleted, deleted_at, ",
        "note, pinned, updated_at"
    );

    pub(super) fn thread_meta_from_row(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<(ThreadMetaKey, ThreadMeta)> {
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
    }
}
mod tags {
    use super::super::super::db::to_io_error;
    use super::super::super::{ThreadMeta, ThreadMetaKey};
    use std::collections::{HashMap, HashSet};
    use std::io;

    pub(super) fn load_tags_into_records(
        connection: &rusqlite::Connection,
        wanted: &HashSet<(&str, &str)>,
        records: &mut HashMap<ThreadMetaKey, ThreadMeta>,
    ) -> io::Result<()> {
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
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(to_io_error)?;

        for row in rows {
            let (agent_type, thread_id, tag) = row.map_err(to_io_error)?;
            if wanted.contains(&(agent_type.as_str(), thread_id.as_str())) {
                records
                    .entry(ThreadMetaKey::new(agent_type, thread_id))
                    .or_default()
                    .tags
                    .push(tag);
            }
        }
        Ok(())
    }

    pub(super) fn hydrate_deleted_tags(
        connection: &rusqlite::Connection,
        deleted: &mut [(ThreadMetaKey, ThreadMeta)],
    ) -> io::Result<()> {
        let wanted = deleted
            .iter()
            .map(|(key, _)| (key.agent_type.as_str(), key.thread_id.as_str()))
            .collect::<HashSet<_>>();
        let mut records = HashMap::new();
        load_tags_into_records(connection, &wanted, &mut records)?;
        for (key, meta) in deleted {
            if let Some(tag_meta) = records.get(key) {
                meta.tags.clone_from(&tag_meta.tags);
            }
        }
        Ok(())
    }
}

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
