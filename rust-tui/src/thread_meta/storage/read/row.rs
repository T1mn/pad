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
