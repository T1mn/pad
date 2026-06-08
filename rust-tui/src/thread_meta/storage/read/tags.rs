use super::super::super::db::to_io_error;
use super::super::super::{ThreadMeta, ThreadMetaKey};
use std::collections::{HashMap, HashSet};
use std::io;

pub(super) fn load_tags_into_records(
    connection: &rusqlite::Connection,
    wanted: &HashSet<ThreadMetaKey>,
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
                ThreadMetaKey::new(row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(to_io_error)?;

    for row in rows {
        let (key, tag) = row.map_err(to_io_error)?;
        if wanted.contains(&key) {
            records.entry(key).or_default().tags.push(tag);
        }
    }
    Ok(())
}

pub(super) fn hydrate_deleted_tags(
    db_path: &std::path::Path,
    deleted: &mut [(ThreadMetaKey, ThreadMeta)],
) -> io::Result<()> {
    let keys = deleted
        .iter()
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let tags = super::load_thread_meta_batch_at(db_path, &keys)?;
    for (key, meta) in deleted {
        if let Some(tag_meta) = tags.get(key) {
            meta.tags = tag_meta.tags.clone();
        }
    }
    Ok(())
}
