use crate::thread_meta::db::{ensure_schema_at, open_db, to_io_error};
use rusqlite::params;
use std::collections::HashSet;
use std::io;
use std::path::Path;

pub(in crate::thread_meta) fn replace_thread_tags_at(
    db_path: &Path,
    agent_type: &str,
    thread_id: &str,
    tags: &[String],
) -> io::Result<()> {
    ensure_schema_at(db_path)?;
    let mut connection = open_db(db_path)?;
    let tx = connection.transaction().map_err(to_io_error)?;
    tx.execute(
        "DELETE FROM thread_tags WHERE agent_type = ?1 AND thread_id = ?2",
        params![agent_type, thread_id],
    )
    .map_err(to_io_error)?;

    insert_unique_tags(&tx, agent_type, thread_id, tags)?;
    tx.commit().map_err(to_io_error)?;
    Ok(())
}

fn insert_unique_tags(
    tx: &rusqlite::Transaction,
    agent_type: &str,
    thread_id: &str,
    tags: &[String],
) -> io::Result<()> {
    let mut seen = HashSet::new();
    let now = crate::app::unix_now_ts();
    for tag in tags {
        let normalized = tag.trim();
        if normalized.is_empty() || !seen.insert(normalized.to_string()) {
            continue;
        }
        tx.execute(
            "INSERT INTO thread_tags (agent_type, thread_id, tag, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![agent_type, thread_id, normalized, now],
        )
        .map_err(to_io_error)?;
    }
    Ok(())
}
