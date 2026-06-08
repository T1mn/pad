use super::super::model::GeminiThreadRecord;
use super::super::util::ensure_parent_dir;
use super::schema::{ensure_schema, open_index_db, to_io_error, DB_TABLE};
use rusqlite::params;
use std::io;
use std::path::Path;

pub(crate) fn replace_records(db_path: &Path, records: &[GeminiThreadRecord]) -> io::Result<()> {
    ensure_parent_dir(db_path)?;
    let mut connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let tx = connection.transaction().map_err(to_io_error)?;

    for record in records {
        upsert_record(&tx, record)?;
    }

    tx.commit().map_err(to_io_error)
}

pub(crate) fn set_threads_archived(
    db_path: &Path,
    session_id: &str,
    archived: bool,
) -> io::Result<()> {
    if !db_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Gemini index not found at {}", db_path.display()),
        ));
    }

    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let archived_value = if archived { 1_i64 } else { 0_i64 };
    let changed = connection
        .execute(
            &format!(
                "UPDATE {DB_TABLE}
                 SET archived = ?2
                 WHERE session_id = ?1"
            ),
            params![session_id, archived_value],
        )
        .map_err(to_io_error)?;

    if changed == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("thread {} was not found", session_id),
        ));
    }

    Ok(())
}

fn upsert_record(tx: &rusqlite::Transaction<'_>, record: &GeminiThreadRecord) -> io::Result<()> {
    tx.execute(
        &format!(
            "INSERT INTO {DB_TABLE} (
                session_id, cwd, project_alias, transcript_path, kind, start_time,
                updated_at, title, subtitle, summary, first_user_message,
                last_user_message, last_assistant_message, archived, has_subagent,
                payload_hash, snapshot_count
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, COALESCE((SELECT archived FROM {DB_TABLE}
                    WHERE session_id = ?1 AND cwd = ?2), 0), ?14,
                ?15, ?16
            )
            ON CONFLICT(session_id, cwd) DO UPDATE SET
                project_alias = excluded.project_alias,
                transcript_path = excluded.transcript_path,
                kind = excluded.kind,
                start_time = excluded.start_time,
                updated_at = excluded.updated_at,
                title = excluded.title,
                subtitle = excluded.subtitle,
                summary = excluded.summary,
                first_user_message = excluded.first_user_message,
                last_user_message = excluded.last_user_message,
                last_assistant_message = excluded.last_assistant_message,
                has_subagent = excluded.has_subagent,
                payload_hash = excluded.payload_hash,
                snapshot_count = excluded.snapshot_count"
        ),
        params![
            record.session_id,
            record.cwd.to_string_lossy().to_string(),
            record.project_alias,
            record.transcript_path.to_string_lossy().to_string(),
            record.kind,
            record.start_time,
            record.updated_at,
            record.title,
            record.subtitle,
            record.summary,
            record.first_user_message,
            record.last_user_message,
            record.last_assistant_message,
            if record.has_subagent { 1_i64 } else { 0_i64 },
            record.payload_hash,
            record.snapshot_count,
        ],
    )
    .map_err(to_io_error)?;
    Ok(())
}
