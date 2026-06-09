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
