use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde_json::Value;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const ACTIVE_THREAD_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;
const CLAUDE_INDEX_DB_FILE: &str = "claude_history.sqlite";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaudeThreadRef {
    pub session_id: String,
    pub cwd: PathBuf,
    pub updated_at: i64,
    pub transcript_path: PathBuf,
    pub title: Option<String>,
    pub archived: bool,
}

#[derive(Clone, Debug)]
struct IndexedClaudeThread {
    session_id: String,
    cwd: PathBuf,
    transcript_path: PathBuf,
    title: Option<String>,
    updated_at: i64,
    last_assistant_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThreadArchiveFilter {
    ActiveOnly,
    ArchivedOnly,
}

pub fn all_threads() -> io::Result<Vec<ClaudeThreadRef>> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    load_threads_at(&root, &db_path, ThreadArchiveFilter::ActiveOnly)
}

pub fn all_archived_threads() -> io::Result<Vec<ClaudeThreadRef>> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    load_threads_at(&root, &db_path, ThreadArchiveFilter::ArchivedOnly)
}

pub fn thread_for_id(session_id: &str) -> io::Result<Option<ClaudeThreadRef>> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    thread_for_id_at(&root, &db_path, session_id)
}

pub fn upsert_hook_session(
    session_id: &str,
    transcript_path: &Path,
    cwd: &Path,
    title: Option<&str>,
    updated_at: i64,
) -> io::Result<()> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    upsert_hook_session_at(
        &root,
        &db_path,
        session_id,
        transcript_path,
        cwd,
        title,
        updated_at,
    )
}

pub fn archive_thread(session_id: &str) -> io::Result<()> {
    mutate_thread_archive_state(session_id, true)
}

pub fn unarchive_thread(session_id: &str) -> io::Result<()> {
    mutate_thread_archive_state(session_id, false)
}

fn default_projects_root() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(".claude").join("projects"))
}

fn default_index_db_path() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(".pad").join(CLAUDE_INDEX_DB_FILE))
}

fn load_threads_at(
    root: &Path,
    db_path: &Path,
    filter: ThreadArchiveFilter,
) -> io::Result<Vec<ClaudeThreadRef>> {
    sync_index_at(root, db_path)?;
    query_threads_at(root, db_path, filter)
}

fn thread_for_id_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
) -> io::Result<Option<ClaudeThreadRef>> {
    let started_at = Instant::now();
    if let Some(thread) = query_thread_for_id_at(root, db_path, session_id)? {
        if started_at.elapsed().as_millis() >= 8 {
            crate::log_debug!(
                "claude_history.lookup: session_id={} hit=index elapsed_ms={}",
                session_id,
                started_at.elapsed().as_millis()
            );
        }
        return Ok(Some(thread));
    }
    sync_index_at(root, db_path)?;
    let result = query_thread_for_id_at(root, db_path, session_id)?;
    if started_at.elapsed().as_millis() >= 20 {
        crate::log_debug!(
            "claude_history.lookup: session_id={} hit_after_sync={} elapsed_ms={}",
            session_id,
            result.is_some(),
            started_at.elapsed().as_millis()
        );
    }
    Ok(result)
}

fn mutate_thread_archive_state(session_id: &str, archive: bool) -> io::Result<()> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    mutate_thread_archive_state_at(&root, &db_path, session_id, archive)
}

fn mutate_thread_archive_state_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
    archive: bool,
) -> io::Result<()> {
    let root_key = normalize_path(root).to_string_lossy().to_string();
    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let updated_at = now_ts();
    let archived_value = if archive { 1_i64 } else { 0_i64 };

    let changed = connection
        .execute(
            "UPDATE claude_threads
             SET archived = ?3,
                 archived_at = CASE WHEN ?3 = 1 THEN ?4 ELSE NULL END,
                 updated_at = MAX(updated_at, ?4)
             WHERE root = ?1
               AND session_id = ?2
               AND archived <> ?3",
            params![root_key, session_id, archived_value, updated_at],
        )
        .map_err(to_io_error)?;
    if changed == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "thread {} was not found or is already in the requested state",
                session_id
            ),
        ));
    }

    Ok(())
}

fn upsert_hook_session_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
    transcript_path: &Path,
    cwd: &Path,
    title: Option<&str>,
    updated_at: i64,
) -> io::Result<()> {
    let started_at = Instant::now();
    let root_key = normalize_path(root).to_string_lossy().to_string();
    let transcript_key = transcript_path.to_string_lossy().to_string();
    let cwd_key = normalize_path(cwd).to_string_lossy().to_string();

    let mut connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let tx = connection.transaction().map_err(to_io_error)?;

    let existing = tx
        .query_row(
            "SELECT transcript_path
             FROM claude_threads
             WHERE root = ?1 AND session_id = ?2
             ORDER BY updated_at DESC, transcript_path DESC
             LIMIT 1",
            params![root_key, session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_io_error)?;

    if let Some(existing_transcript) = existing {
        tx.execute(
            "UPDATE claude_threads
             SET transcript_path = ?3,
                 cwd = ?4,
                 title = COALESCE(?5, title),
                 updated_at = MAX(updated_at, ?6),
                 last_assistant_at = MAX(last_assistant_at, ?6),
                 file_mtime = MAX(file_mtime, ?6),
                 last_seen_at = ?6
             WHERE root = ?1 AND transcript_path = ?2",
            params![
                root_key,
                existing_transcript,
                transcript_key,
                cwd_key,
                title,
                updated_at,
            ],
        )
        .map_err(to_io_error)?;
    } else {
        tx.execute(
            "INSERT INTO claude_threads (
                root, transcript_path, session_id, cwd, title,
                updated_at, last_assistant_at, file_mtime, last_seen_seq,
                last_seen_at, is_sidechain
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?6, 0, ?6, 0)",
            params![
                root_key,
                transcript_key,
                session_id,
                cwd_key,
                title,
                updated_at
            ],
        )
        .map_err(to_io_error)?;
    }

    tx.commit().map_err(to_io_error)?;
    if started_at.elapsed().as_millis() >= 8 {
        crate::log_debug!(
            "claude_history.hook_upsert: session_id={} elapsed_ms={} transcript={}",
            session_id,
            started_at.elapsed().as_millis(),
            transcript_path.display()
        );
    }
    Ok(())
}

fn sync_index_at(root: &Path, db_path: &Path) -> io::Result<()> {
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

fn ensure_schema(connection: &Connection) -> io::Result<()> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS claude_threads (
                root TEXT NOT NULL,
                transcript_path TEXT NOT NULL,
                session_id TEXT NOT NULL,
                cwd TEXT NOT NULL,
                title TEXT,
                updated_at INTEGER NOT NULL,
                last_assistant_at INTEGER NOT NULL,
                file_mtime INTEGER NOT NULL,
                last_seen_seq INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL,
                is_sidechain INTEGER NOT NULL DEFAULT 0,
                archived INTEGER NOT NULL DEFAULT 0,
                archived_at INTEGER,
                PRIMARY KEY(root, transcript_path)
            );
            CREATE INDEX IF NOT EXISTS idx_claude_threads_root_session
                ON claude_threads(root, session_id, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_claude_threads_root_cwd
                ON claude_threads(root, cwd, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_claude_threads_root_activity
                ON claude_threads(root, last_assistant_at DESC, updated_at DESC);
            CREATE TABLE IF NOT EXISTS claude_scan_state (
                root TEXT PRIMARY KEY,
                scan_seq INTEGER NOT NULL,
                last_indexed_at INTEGER NOT NULL
            );",
        )
        .map_err(to_io_error)?;
    ensure_optional_column(
        connection,
        "claude_threads",
        "archived",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_optional_column(connection, "claude_threads", "archived_at", "INTEGER")?;
    Ok(())
}

fn ensure_optional_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> io::Result<()> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(to_io_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(to_io_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_io_error)?;
    if columns.iter().any(|existing| existing == column) {
        return Ok(());
    }
    connection
        .execute(
            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
            [],
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn open_index_db(db_path: &Path) -> io::Result<Connection> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;

    connection
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
        .map_err(to_io_error)?;
    Ok(connection)
}

fn next_scan_seq(tx: &rusqlite::Transaction<'_>, root_key: &str) -> io::Result<i64> {
    let current = tx
        .query_row(
            "SELECT scan_seq FROM claude_scan_state WHERE root = ?1",
            [root_key],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(to_io_error)?
        .unwrap_or(0);
    Ok(current.saturating_add(1))
}

fn upsert_thread_row(
    tx: &rusqlite::Transaction<'_>,
    root_key: &str,
    thread: &IndexedClaudeThread,
    file_mtime: i64,
    scan_seq: i64,
) -> io::Result<()> {
    tx.execute(
        "INSERT INTO claude_threads (
            root, transcript_path, session_id, cwd, title,
            updated_at, last_assistant_at, file_mtime, last_seen_seq,
            last_seen_at, is_sidechain
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)
         ON CONFLICT(root, transcript_path) DO UPDATE SET
            session_id = excluded.session_id,
            cwd = excluded.cwd,
            title = excluded.title,
            updated_at = excluded.updated_at,
            last_assistant_at = excluded.last_assistant_at,
            file_mtime = excluded.file_mtime,
            last_seen_seq = excluded.last_seen_seq,
            last_seen_at = excluded.last_seen_at,
               is_sidechain = excluded.is_sidechain",
        params![
            root_key,
            thread.transcript_path.to_string_lossy().to_string(),
            thread.session_id,
            normalize_path(&thread.cwd).to_string_lossy().to_string(),
            thread.title,
            thread.updated_at,
            thread.last_assistant_at,
            file_mtime,
            scan_seq,
            now_ts(),
        ],
    )
    .map_err(to_io_error)?;

    Ok(())
}

fn discover_thread_files(root: &Path) -> io::Result<Vec<PathBuf>> {
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

fn query_threads_at(
    root: &Path,
    db_path: &Path,
    filter: ThreadArchiveFilter,
) -> io::Result<Vec<ClaudeThreadRef>> {
    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let root_key = normalize_path(root).to_string_lossy().to_string();
    let min_assistant_ts = now_ts().saturating_sub(ACTIVE_THREAD_MAX_AGE_SECS);

    let sql = match filter {
        ThreadArchiveFilter::ActiveOnly => {
            "SELECT session_id, cwd, updated_at, transcript_path, title, archived
             FROM claude_threads
             WHERE root = ?1
               AND archived = 0
               AND last_assistant_at >= ?2
             ORDER BY updated_at DESC, transcript_path DESC"
        }
        ThreadArchiveFilter::ArchivedOnly => {
            "SELECT session_id, cwd, updated_at, transcript_path, title, archived
             FROM claude_threads
             WHERE root = ?1
               AND archived = 1
             ORDER BY updated_at DESC, transcript_path DESC"
        }
    };
    let mut statement = connection.prepare(sql).map_err(to_io_error)?;
    let rows = match filter {
        ThreadArchiveFilter::ActiveOnly => {
            statement.query_map(params![root_key, min_assistant_ts], map_thread_row)
        }
        ThreadArchiveFilter::ArchivedOnly => statement.query_map(params![root_key], map_thread_row),
    }
    .map_err(to_io_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_io_error)
}

fn query_thread_for_id_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
) -> io::Result<Option<ClaudeThreadRef>> {
    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let root_key = normalize_path(root).to_string_lossy().to_string();

    connection
        .query_row(
            "SELECT session_id, cwd, updated_at, transcript_path, title, archived
             FROM claude_threads
             WHERE root = ?1
               AND session_id = ?2
             ORDER BY updated_at DESC, transcript_path DESC
             LIMIT 1",
            params![root_key, session_id],
            map_thread_row,
        )
        .optional()
        .map_err(to_io_error)
}

fn map_thread_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClaudeThreadRef> {
    Ok(ClaudeThreadRef {
        session_id: row.get::<_, String>(0)?,
        cwd: PathBuf::from(row.get::<_, String>(1)?),
        updated_at: row.get::<_, i64>(2)?,
        transcript_path: PathBuf::from(row.get::<_, String>(3)?),
        title: row.get::<_, Option<String>>(4)?,
        archived: row.get::<_, i64>(5).unwrap_or_default() != 0,
    })
}

fn file_mtime_secs(path: &Path) -> io::Result<i64> {
    fs::metadata(path)?
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "failed to read file mtime"))
}

fn parse_claude_thread_file(path: &Path) -> io::Result<Option<IndexedClaudeThread>> {
    #[cfg(test)]
    {
        PARSE_THREAD_FILE_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    if path
        .components()
        .any(|component| component.as_os_str().to_string_lossy() == "subagents")
    {
        return Ok(None);
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut session_id = None;
    let mut cwd = None;
    let mut title = None;
    let mut has_dialogue_event = false;
    let mut last_assistant_ts = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value
            .get("isSidechain")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Ok(None);
        }

        if session_id.is_none() {
            session_id = value
                .get("sessionId")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
        if cwd.is_none() {
            cwd = value.get("cwd").and_then(Value::as_str).map(PathBuf::from);
        }
        if title.is_none() {
            title = extract_first_user_prompt(&value);
        }
        if matches!(
            value.get("type").and_then(Value::as_str),
            Some("user" | "assistant")
        ) {
            has_dialogue_event = true;
        }
        if value.get("type").and_then(Value::as_str) == Some("assistant") {
            if let Some(timestamp) = value
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(parse_rfc3339_utc_ts)
            {
                last_assistant_ts = Some(last_assistant_ts.unwrap_or(timestamp).max(timestamp));
            }
        }
    }

    let Some(session_id) = session_id else {
        return Ok(None);
    };
    let Some(cwd) = cwd else {
        return Ok(None);
    };
    if !has_dialogue_event {
        return Ok(None);
    }

    let updated_at = file_mtime_secs(path)?;
    let last_assistant_at = last_assistant_ts.unwrap_or(updated_at);

    Ok(Some(IndexedClaudeThread {
        session_id,
        cwd,
        transcript_path: path.to_path_buf(),
        title,
        updated_at,
        last_assistant_at,
    }))
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn parse_rfc3339_utc_ts(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    let year = parse_i32(bytes, 0, 4)?;
    let month = parse_u32(bytes, 5, 7)?;
    let day = parse_u32(bytes, 8, 10)?;
    let hour = parse_u32(bytes, 11, 13)?;
    let minute = parse_u32(bytes, 14, 16)?;
    let second = parse_u32(bytes, 17, 19)?;
    let tz_start = text[19..]
        .find(['Z', '+', '-'])
        .map(|idx| idx + 19)
        .unwrap_or(bytes.len());
    let offset_secs = if bytes.get(tz_start) == Some(&b'Z') || tz_start == bytes.len() {
        0
    } else {
        let sign = if bytes.get(tz_start) == Some(&b'-') {
            -1_i64
        } else {
            1_i64
        };
        let offset_hour = parse_u32(bytes, tz_start + 1, tz_start + 3)? as i64;
        let offset_minute = parse_u32(bytes, tz_start + 4, tz_start + 6)? as i64;
        sign * (offset_hour * 3600 + offset_minute * 60)
    };

    let days = days_from_civil(year, month, day)?;
    Some(days * 86_400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64 - offset_secs)
}

fn parse_i32(bytes: &[u8], start: usize, end: usize) -> Option<i32> {
    std::str::from_utf8(bytes.get(start..end)?)
        .ok()?
        .parse()
        .ok()
}

fn parse_u32(bytes: &[u8], start: usize, end: usize) -> Option<u32> {
    std::str::from_utf8(bytes.get(start..end)?)
        .ok()?
        .parse()
        .ok()
}

fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let adjust = if month <= 2 { 1 } else { 0 };
    let year = year - adjust;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month as i32 + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era as i64 * 146_097 + doe as i64 - 719_468)
}

fn extract_first_user_prompt(value: &Value) -> Option<String> {
    if value.get("type").and_then(Value::as_str) != Some("user") {
        return None;
    }

    let message = value.get("message")?;
    if message.get("role").and_then(Value::as_str) != Some("user") {
        return None;
    }

    match message.get("content") {
        Some(Value::String(text)) => clean_text(text),
        Some(Value::Array(items)) => items.iter().find_map(|item| {
            if item.get("type").and_then(Value::as_str) != Some("text") {
                return None;
            }
            item.get("text")
                .and_then(Value::as_str)
                .and_then(clean_text)
        }),
        _ => None,
    }
}

fn clean_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || is_local_command_scaffold(trimmed) {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_local_command_scaffold(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("<local-command-caveat>")
        || lowered.contains("</local-command-caveat>")
        || lowered.contains("<command-name>")
        || lowered.contains("</command-name>")
        || lowered.contains("<command-message>")
        || lowered.contains("</command-message>")
        || lowered.contains("<command-args>")
        || lowered.contains("</command-args>")
}

fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

#[cfg(test)]
static PARSE_THREAD_FILE_CALLS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::thread::sleep;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pad-claude-history-{}-{}", name, stamp));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn temp_db(name: &str) -> std::path::PathBuf {
        temp_dir(name).join("claude.sqlite")
    }

    fn write_thread(file: &Path, session_id: &str, cwd: &str, title: &str, assistant_ts: &str) {
        fs::write(
            file,
            format!(
                concat!(
                    "{{\"type\":\"user\",\"sessionId\":\"{}\",\"cwd\":\"{}\",\"message\":{{\"role\":\"user\",\"content\":\"{}\"}}}}\n",
                    "{{\"type\":\"assistant\",\"sessionId\":\"{}\",\"cwd\":\"{}\",\"message\":{{\"role\":\"assistant\",\"content\":\"ok\"}},\"timestamp\":\"{}\"}}\n"
                ),
                session_id, cwd, title, session_id, cwd, assistant_ts
            ),
        )
        .unwrap();
    }

    #[test]
    fn parse_claude_thread_file_extracts_session_cwd_and_title() {
        let dir = temp_dir("single");
        let file = dir.join("sample.jsonl");
        write_thread(
            &file,
            "abc",
            "/tmp/project",
            "first prompt",
            "2099-03-10T05:41:54.280Z",
        );

        let parsed = parse_claude_thread_file(&file).unwrap().unwrap();
        fs::remove_dir_all(&dir).ok();

        assert_eq!(parsed.session_id, "abc");
        assert_eq!(parsed.cwd, Path::new("/tmp/project"));
        assert_eq!(parsed.title.as_deref(), Some("first prompt"));
    }

    #[test]
    fn incremental_sync_skips_unchanged_files_and_removes_deleted_ones() {
        let root = temp_dir("incremental");
        let db = temp_db("incremental");
        let file_a = root.join("a.jsonl");
        let file_b = root.join("b.jsonl");
        write_thread(
            &file_a,
            "a",
            "/tmp/project-a",
            "prompt a",
            "2099-03-10T05:41:54.280Z",
        );

        sync_index_at(&root, &db).unwrap();
        let first = load_threads_at(&root, &db, ThreadArchiveFilter::ActiveOnly).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].session_id, "a");

        sleep(Duration::from_millis(1100));
        write_thread(
            &file_b,
            "b",
            "/tmp/project-b",
            "prompt b",
            "2099-03-10T05:41:54.280Z",
        );
        sync_index_at(&root, &db).unwrap();
        let second = load_threads_at(&root, &db, ThreadArchiveFilter::ActiveOnly).unwrap();
        assert_eq!(second.len(), 2);
        assert_eq!(second[0].session_id, "b");
        assert_eq!(second[1].session_id, "a");

        fs::remove_file(&file_a).unwrap();
        sync_index_at(&root, &db).unwrap();
        let third = load_threads_at(&root, &db, ThreadArchiveFilter::ActiveOnly).unwrap();
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].session_id, "b");

        fs::remove_dir_all(&root).ok();
        fs::remove_file(&db).ok();
    }

    #[test]
    fn thread_lookup_works_without_active_filtering() {
        let root = temp_dir("lookup");
        let db = temp_db("lookup");
        let file = root.join("stale.jsonl");
        write_thread(
            &file,
            "stale",
            "/tmp/project",
            "prompt",
            "2020-03-10T05:41:54.280Z",
        );

        sync_index_at(&root, &db).unwrap();
        let lookup = thread_for_id_at(&root, &db, "stale").unwrap();
        let active = query_threads_at(&root, &db, ThreadArchiveFilter::ActiveOnly).unwrap();

        assert!(lookup.is_some());
        assert!(active.is_empty());

        fs::remove_dir_all(&root).ok();
        fs::remove_file(&db).ok();
    }

    #[test]
    fn hook_upsert_inserts_session_when_index_is_empty() {
        let root = temp_dir("hook-upsert");
        let db = temp_db("hook-upsert");
        let transcript = root.join("hook.jsonl");
        let cwd = root.join("workspace");
        fs::create_dir_all(&cwd).unwrap();

        upsert_hook_session_at(
            &root,
            &db,
            "hook-session",
            &transcript,
            &cwd,
            Some("hook title"),
            1_700_000_000,
        )
        .unwrap();

        let lookup = thread_for_id_at(&root, &db, "hook-session")
            .unwrap()
            .unwrap();
        assert_eq!(lookup.session_id, "hook-session");
        assert_eq!(lookup.transcript_path, transcript);
        assert_eq!(lookup.cwd, normalize_path(&cwd));
        assert_eq!(lookup.title.as_deref(), Some("hook title"));
        assert!(!lookup.archived);

        fs::remove_dir_all(&root).ok();
        fs::remove_file(&db).ok();
    }

    #[test]
    fn archived_threads_are_excluded_from_active_list_and_visible_in_archived_list() {
        let root = temp_dir("archived-filter");
        let db = temp_db("archived-filter");
        let file = root.join("main.jsonl");
        write_thread(
            &file,
            "main",
            "/tmp/project",
            "prompt",
            "2099-03-10T05:41:54.280Z",
        );

        sync_index_at(&root, &db).unwrap();
        mutate_thread_archive_state_at(&root, &db, "main", true).unwrap();

        let active = query_threads_at(&root, &db, ThreadArchiveFilter::ActiveOnly).unwrap();
        let archived = query_threads_at(&root, &db, ThreadArchiveFilter::ArchivedOnly).unwrap();
        let lookup = thread_for_id_at(&root, &db, "main").unwrap().unwrap();

        assert!(active.is_empty());
        assert_eq!(archived.len(), 1);
        assert!(archived[0].archived);
        assert!(lookup.archived);

        fs::remove_dir_all(&root).ok();
        fs::remove_file(&db).ok();
    }

    #[test]
    fn progress_only_stub_file_is_filtered_out() {
        let dir = temp_dir("progress-only");
        let file = dir.join("stub.jsonl");
        fs::write(
            &file,
            "{\"type\":\"progress\",\"sessionId\":\"stub\",\"cwd\":\"/tmp/project\",\"data\":{\"type\":\"hook_progress\"}}\n",
        )
        .unwrap();

        let parsed = parse_claude_thread_file(&file).unwrap();
        fs::remove_dir_all(&dir).ok();

        assert!(parsed.is_none());
    }

    #[test]
    fn sidechain_file_is_filtered_out() {
        let dir = temp_dir("sidechain");
        let file = dir.join("agent-sidechain.jsonl");
        fs::write(
            &file,
            concat!(
                "{\"type\":\"user\",\"isSidechain\":true,\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"subagent\"}}\n",
                "{\"type\":\"assistant\",\"isSidechain\":true,\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"assistant\",\"content\":\"ok\"}}\n"
            ),
        )
        .unwrap();

        let parsed = parse_claude_thread_file(&file).unwrap();
        fs::remove_dir_all(&dir).ok();

        assert!(parsed.is_none());
    }

    #[test]
    fn local_command_scaffold_is_not_used_as_title() {
        let dir = temp_dir("local-command");
        let file = dir.join("main.jsonl");
        fs::write(
            &file,
            concat!(
                "{\"type\":\"user\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"<local-command-caveat>do not use</local-command-caveat>\"}}\n",
                "{\"type\":\"user\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"<command-name>/clear</command-name><command-message>clear</command-message>\"}}\n",
                "{\"type\":\"user\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"真实用户问题\"}}\n",
                "{\"type\":\"assistant\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"assistant\",\"content\":\"ok\"}}\n"
            ),
        )
        .unwrap();

        let parsed = parse_claude_thread_file(&file).unwrap().unwrap();
        fs::remove_dir_all(&dir).ok();

        assert_eq!(parsed.title.as_deref(), Some("真实用户问题"));
    }

    #[test]
    fn read_threads_ignores_subagents_directory() {
        let root = temp_dir("subagents-dir");
        let main_file = root.join("main.jsonl");
        write_thread(
            &main_file,
            "main",
            "/tmp/project",
            "main prompt",
            "2099-03-10T05:41:54.280Z",
        );

        let sub_dir = root.join("subagents");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(
            sub_dir.join("agent-a79dd02e.jsonl"),
            concat!(
                "{\"type\":\"user\",\"isSidechain\":true,\"sessionId\":\"main\",\"cwd\":\"/tmp/worktree\",\"message\":{\"role\":\"user\",\"content\":\"sidechain\"}}\n",
                "{\"type\":\"assistant\",\"isSidechain\":true,\"sessionId\":\"main\",\"cwd\":\"/tmp/worktree\",\"message\":{\"role\":\"assistant\",\"content\":\"ok\"}}\n"
            ),
        )
        .unwrap();

        let result = discover_thread_files(&root).unwrap();
        fs::remove_dir_all(&root).ok();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].file_name().and_then(|name| name.to_str()),
            Some("main.jsonl")
        );
    }

    #[test]
    fn stale_threads_without_recent_assistant_are_filtered_out() {
        let root = temp_dir("stale-filter");
        let db = temp_db("stale-filter-db");
        let file = root.join("stale.jsonl");
        write_thread(
            &file,
            "old",
            "/tmp/project",
            "hello",
            "2020-03-10T05:41:54.280Z",
        );

        sync_index_at(&root, &db).unwrap();
        let active = load_threads_at(&root, &db, ThreadArchiveFilter::ActiveOnly).unwrap();
        let lookup = thread_for_id_at(&root, &db, "old").unwrap();

        assert!(active.is_empty());
        assert!(lookup.is_some());

        fs::remove_dir_all(&root).ok();
        fs::remove_file(&db).ok();
    }
}
