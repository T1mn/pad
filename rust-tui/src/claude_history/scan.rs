use super::db::{ensure_schema, next_scan_seq, open_index_db, upsert_thread_row};
use super::parse::parse_claude_thread_file;
use super::util::{file_mtime_secs, normalize_path, now_ts, to_io_error};
use rusqlite::{params, OptionalExtension};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(crate) fn discover_thread_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == "subagents")
                {
                    continue;
                }
                stack.push(path);
                continue;
            }

            if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }
    }

    Ok(files)
}

pub(crate) fn sync_index_at(root: &Path, db_path: &Path) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }

    let started_at = Instant::now();
    let root_key = normalize_path(root).to_string_lossy().to_string();
    let mut connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;

    let mut discovered = discover_thread_files(root)?;
    discovered.sort();
    let discovered_count = discovered.len();
    let mut reused_count = 0usize;
    let mut reparsed_count = 0usize;
    let mut deleted_count = 0usize;

    let tx = connection.transaction().map_err(to_io_error)?;
    let scan_seq = next_scan_seq(&tx, &root_key)?;

    for transcript_path in discovered {
        let file_mtime = file_mtime_secs(&transcript_path)?;
        let existing_mtime = tx
            .query_row(
                "SELECT file_mtime
                 FROM claude_threads
                 WHERE root = ?1 AND transcript_path = ?2",
                params![root_key, transcript_path.to_string_lossy().to_string()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(to_io_error)?;

        if existing_mtime == Some(file_mtime) {
            reused_count += 1;
            tx.execute(
                "UPDATE claude_threads
                 SET last_seen_seq = ?3, last_seen_at = ?4
                 WHERE root = ?1 AND transcript_path = ?2",
                params![
                    root_key,
                    transcript_path.to_string_lossy().to_string(),
                    scan_seq,
                    now_ts()
                ],
            )
            .map_err(to_io_error)?;
            continue;
        }

        match parse_claude_thread_file(&transcript_path)? {
            Some(parsed) => {
                reparsed_count += 1;
                upsert_thread_row(&tx, &root_key, &parsed, file_mtime, scan_seq)?
            }
            None => {
                deleted_count += 1;
                tx.execute(
                    "DELETE FROM claude_threads
                     WHERE root = ?1 AND transcript_path = ?2",
                    params![root_key, transcript_path.to_string_lossy().to_string()],
                )
                .map_err(to_io_error)?;
            }
        }
    }

    tx.execute(
        "DELETE FROM claude_threads
         WHERE root = ?1 AND last_seen_seq <> ?2",
        params![root_key, scan_seq],
    )
    .map_err(to_io_error)?;

    tx.execute(
        "INSERT INTO claude_scan_state(root, scan_seq, last_indexed_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(root) DO UPDATE SET
             scan_seq = excluded.scan_seq,
             last_indexed_at = excluded.last_indexed_at",
        params![root_key, scan_seq, now_ts()],
    )
    .map_err(to_io_error)?;

    tx.commit().map_err(to_io_error)?;
    if started_at.elapsed().as_millis() >= 20 {
        crate::log_debug!(
            "claude_history.sync: root={} elapsed_ms={} files={} reparsed={} reused={} deleted={}",
            root.display(),
            started_at.elapsed().as_millis(),
            discovered_count,
            reparsed_count,
            reused_count,
            deleted_count
        );
    }
    Ok(())
}
