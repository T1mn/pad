use super::super::db::{ensure_schema_at, open_db, to_io_error};
use rusqlite::params;
use std::collections::HashSet;
use std::io;
use std::path::Path;

pub(in crate::thread_meta) fn upsert_thread_meta_at(
    db_path: &Path,
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

pub(in crate::thread_meta) fn upsert_generated_title_at(
    db_path: &Path,
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

pub(in crate::thread_meta) fn set_thread_deleted_at(
    db_path: &Path,
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
