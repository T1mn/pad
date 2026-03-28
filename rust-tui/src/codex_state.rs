use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const THREAD_CACHE_TTL: Duration = Duration::from_secs(10);
const ACTIVE_THREAD_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexThreadRef {
    pub thread_id: String,
    pub cwd: PathBuf,
    pub updated_at: i64,
    pub rollout_path: PathBuf,
    pub title: Option<String>,
    pub first_user_message: Option<String>,
    pub source: Option<String>,
    pub archived: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThreadArchiveFilter {
    ActiveOnly,
    ArchivedOnly,
    All,
}

#[derive(Clone)]
struct CachedThreads {
    loaded_at: Instant,
    threads: Vec<CodexThreadRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    db_path: PathBuf,
    filter: ThreadArchiveFilter,
}

static THREAD_CACHE: OnceLock<Mutex<HashMap<CacheKey, CachedThreads>>> = OnceLock::new();

pub fn latest_thread_for_cwd(cwd: &Path) -> io::Result<Option<CodexThreadRef>> {
    let threads = load_threads(default_db_path()?, ThreadArchiveFilter::ActiveOnly)?;
    Ok(select_latest_thread_for_cwd(cwd, &threads).cloned())
}

pub fn all_threads() -> io::Result<Vec<CodexThreadRef>> {
    load_threads(default_db_path()?, ThreadArchiveFilter::ActiveOnly)
}

pub fn all_archived_threads() -> io::Result<Vec<CodexThreadRef>> {
    load_threads(default_db_path()?, ThreadArchiveFilter::ArchivedOnly)
}

pub fn threads_for_cwd(cwd: &Path) -> io::Result<Vec<CodexThreadRef>> {
    let threads = load_threads(default_db_path()?, ThreadArchiveFilter::ActiveOnly)?;
    let normalized = normalize_path(cwd);
    Ok(threads
        .into_iter()
        .filter(|thread| normalize_path(&thread.cwd) == normalized)
        .collect())
}

pub fn archived_threads_for_cwd(cwd: &Path) -> io::Result<Vec<CodexThreadRef>> {
    let threads = load_threads(default_db_path()?, ThreadArchiveFilter::ArchivedOnly)?;
    let normalized = normalize_path(cwd);
    Ok(threads
        .into_iter()
        .filter(|thread| normalize_path(&thread.cwd) == normalized)
        .collect())
}

pub fn thread_for_id(thread_id: &str) -> io::Result<Option<CodexThreadRef>> {
    read_thread_for_id(&default_db_path()?, thread_id)
}

pub fn subagent_parent_thread_id(thread_id: &str) -> io::Result<Option<String>> {
    let Some(thread) = thread_for_id(thread_id)? else {
        return Ok(None);
    };
    Ok(parse_subagent_parent_thread_id(thread.source.as_deref()))
}

pub fn archive_thread(thread_id: &str) -> io::Result<()> {
    mutate_thread_archive_state(thread_id, true)
}

pub fn unarchive_thread(thread_id: &str) -> io::Result<()> {
    mutate_thread_archive_state(thread_id, false)
}

fn default_db_path() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(".codex").join("state_5.sqlite"))
}

fn load_threads(db_path: PathBuf, filter: ThreadArchiveFilter) -> io::Result<Vec<CodexThreadRef>> {
    let cache_key = CacheKey {
        db_path: db_path.clone(),
        filter,
    };
    let cache = THREAD_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.get(&cache_key) {
            if cached.loaded_at.elapsed() < THREAD_CACHE_TTL {
                return Ok(cached.threads.clone());
            }
        }
    }

    let threads = read_threads_from_db(&db_path, filter)?;
    if let Ok(mut guard) = cache.lock() {
        guard.insert(
            cache_key,
            CachedThreads {
                loaded_at: Instant::now(),
                threads: threads.clone(),
            },
        );
    }
    Ok(threads)
}

fn read_threads_from_db(
    db_path: &Path,
    filter: ThreadArchiveFilter,
) -> io::Result<Vec<CodexThreadRef>> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;

    let min_updated_at = unix_now_ts().saturating_sub(ACTIVE_THREAD_MAX_AGE_SECS);
    let sql = match filter {
        ThreadArchiveFilter::ActiveOnly => {
            "SELECT id, cwd, updated_at, rollout_path, title, first_user_message, source, archived
             FROM threads
             WHERE rollout_path IS NOT NULL
               AND TRIM(rollout_path) <> ''
               AND archived = 0
               AND updated_at >= ?1
             ORDER BY updated_at DESC, id DESC"
        }
        ThreadArchiveFilter::ArchivedOnly => {
            "SELECT id, cwd, updated_at, rollout_path, title, first_user_message, source, archived
             FROM threads
             WHERE rollout_path IS NOT NULL
               AND TRIM(rollout_path) <> ''
               AND archived = 1
             ORDER BY updated_at DESC, id DESC"
        }
        ThreadArchiveFilter::All => {
            "SELECT id, cwd, updated_at, rollout_path, title, first_user_message, source, archived
             FROM threads
             WHERE rollout_path IS NOT NULL
               AND TRIM(rollout_path) <> ''
             ORDER BY updated_at DESC, id DESC"
        }
    };
    let mut statement = connection.prepare(sql).map_err(to_io_error)?;
    let mapper = |row: &rusqlite::Row<'_>| {
        Ok(CodexThreadRef {
            thread_id: row.get::<_, String>(0)?,
            cwd: PathBuf::from(row.get::<_, String>(1)?),
            updated_at: row.get::<_, i64>(2)?,
            rollout_path: PathBuf::from(row.get::<_, String>(3)?),
            title: row.get::<_, Option<String>>(4)?,
            first_user_message: row.get::<_, Option<String>>(5)?,
            source: row.get::<_, Option<String>>(6)?,
            archived: row.get::<_, i64>(7).unwrap_or_default() != 0,
        })
    };
    let rows = match filter {
        ThreadArchiveFilter::ActiveOnly => statement.query_map([min_updated_at], mapper),
        ThreadArchiveFilter::ArchivedOnly | ThreadArchiveFilter::All => {
            statement.query_map([], mapper)
        }
    }
    .map_err(to_io_error)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(to_io_error)
}

fn read_thread_for_id(db_path: &Path, thread_id: &str) -> io::Result<Option<CodexThreadRef>> {
    if !db_path.exists() {
        return Ok(None);
    }

    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;

    connection
        .query_row(
            "SELECT id, cwd, updated_at, rollout_path, title, first_user_message, source, archived
             FROM threads
             WHERE id = ?1
               AND rollout_path IS NOT NULL
               AND TRIM(rollout_path) <> ''",
            [thread_id],
            |row| {
                Ok(CodexThreadRef {
                    thread_id: row.get::<_, String>(0)?,
                    cwd: PathBuf::from(row.get::<_, String>(1)?),
                    updated_at: row.get::<_, i64>(2)?,
                    rollout_path: PathBuf::from(row.get::<_, String>(3)?),
                    title: row.get::<_, Option<String>>(4)?,
                    first_user_message: row.get::<_, Option<String>>(5)?,
                    source: row.get::<_, Option<String>>(6)?,
                    archived: row.get::<_, i64>(7).unwrap_or_default() != 0,
                })
            },
        )
        .optional()
        .map_err(to_io_error)
}

fn parse_subagent_parent_thread_id(source: Option<&str>) -> Option<String> {
    let source = source?.trim();
    if source.is_empty() || !source.starts_with('{') {
        return None;
    }

    let value = serde_json::from_str::<Value>(source).ok()?;
    value
        .get("subagent")
        .and_then(|subagent| subagent.get("thread_spawn"))
        .and_then(|spawn| spawn.get("parent_thread_id"))
        .and_then(|parent| parent.as_str())
        .map(|parent| parent.to_string())
}

fn mutate_thread_archive_state(thread_id: &str, archive: bool) -> io::Result<()> {
    let db_path = default_db_path()?;
    let codex_home = codex_home_dir()?;
    mutate_thread_archive_state_at(&db_path, &codex_home, thread_id, archive)
}

fn mutate_thread_archive_state_at(
    db_path: &Path,
    codex_home: &Path,
    thread_id: &str,
    archive: bool,
) -> io::Result<()> {
    let connection = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;

    let thread = read_thread_for_update(&connection, thread_id)?;
    if archive && thread.archived {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("thread {} is already archived", thread_id),
        ));
    }
    if !archive && !thread.archived {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("thread {} is not archived", thread_id),
        ));
    }

    let source_path = PathBuf::from(&thread.rollout_path);
    let file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "rollout path missing file name")
        })?;
    if !file_name.contains(thread_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "rollout path `{}` does not match thread id {}",
                source_path.display(),
                thread_id
            ),
        ));
    }

    let target_path = if archive {
        ensure_path_in_dir(&source_path, &codex_home.join("sessions"), "sessions")?;
        codex_home.join("archived_sessions").join(file_name)
    } else {
        ensure_path_in_dir(
            &source_path,
            &codex_home.join("archived_sessions"),
            "archived directory",
        )?;
        let (year, month, day) = rollout_date_parts(file_name)?;
        codex_home
            .join("sessions")
            .join(year)
            .join(month)
            .join(day)
            .join(file_name)
    };

    let target_parent = target_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "target rollout path missing parent",
        )
    })?;
    fs::create_dir_all(target_parent)?;
    if target_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("target rollout already exists: {}", target_path.display()),
        ));
    }
    if !source_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("rollout file not found: {}", source_path.display()),
        ));
    }

    fs::rename(&source_path, &target_path)?;

    let update_result = if archive {
        let archived_at = unix_now_ts();
        connection.execute(
            "UPDATE threads
             SET archived = 1, archived_at = ?1, rollout_path = ?2
             WHERE id = ?3 AND archived = 0",
            (
                archived_at,
                target_path.to_string_lossy().to_string(),
                thread_id.to_string(),
            ),
        )
    } else {
        let updated_at = unix_now_ts();
        connection.execute(
            "UPDATE threads
             SET archived = 0, archived_at = NULL, rollout_path = ?1, updated_at = ?2
             WHERE id = ?3 AND archived = 1",
            (
                target_path.to_string_lossy().to_string(),
                updated_at,
                thread_id.to_string(),
            ),
        )
    };

    match update_result.map_err(to_io_error) {
        Ok(1) => {
            invalidate_thread_cache(&db_path);
            Ok(())
        }
        Ok(_) => {
            let _ = fs::rename(&target_path, &source_path);
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("failed to update thread {} archive state", thread_id),
            ))
        }
        Err(err) => {
            let _ = fs::rename(&target_path, &source_path);
            Err(err)
        }
    }
}

fn read_thread_for_update(connection: &Connection, thread_id: &str) -> io::Result<ThreadRow> {
    connection
        .query_row(
            "SELECT rollout_path, archived FROM threads WHERE id = ?1",
            [thread_id],
            |row| {
                Ok(ThreadRow {
                    rollout_path: row.get::<_, String>(0)?,
                    archived: row.get::<_, i64>(1)? != 0,
                })
            },
        )
        .map_err(to_io_error)
}

fn ensure_path_in_dir(path: &Path, dir: &Path, label: &str) -> io::Result<()> {
    if !path.starts_with(dir) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("rollout path `{}` must be in {}", path.display(), label),
        ));
    }
    Ok(())
}

fn rollout_date_parts(file_name: &str) -> io::Result<(&str, &str, &str)> {
    let stem = file_name.strip_prefix("rollout-").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "rollout path missing filename timestamp",
        )
    })?;
    if stem.len() < 10 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "rollout path missing filename timestamp",
        ));
    }

    let year = &stem[0..4];
    let month = &stem[5..7];
    let day = &stem[8..10];
    if stem.as_bytes().get(4) != Some(&b'-') || stem.as_bytes().get(7) != Some(&b'-') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "rollout path missing filename timestamp",
        ));
    }
    Ok((year, month, day))
}

fn codex_home_dir() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(".codex"))
}

fn invalidate_thread_cache(db_path: &Path) {
    let Some(cache) = THREAD_CACHE.get() else {
        return;
    };
    if let Ok(mut guard) = cache.lock() {
        guard.retain(|key, _| key.db_path != db_path);
    }
}

fn unix_now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

struct ThreadRow {
    rollout_path: String,
    archived: bool,
}

fn select_latest_thread_for_cwd<'a>(
    cwd: &Path,
    threads: &'a [CodexThreadRef],
) -> Option<&'a CodexThreadRef> {
    let normalized = normalize_path(cwd);

    threads
        .iter()
        .filter(|thread| normalize_path(&thread.cwd) == normalized)
        .max_by_key(|thread| thread.updated_at)
        .or_else(|| {
            threads
                .iter()
                .filter_map(|thread| {
                    let thread_cwd = normalize_path(&thread.cwd);
                    relation_score(&normalized, &thread_cwd).map(|score| (score, thread))
                })
                .max_by_key(|(score, thread)| (*score, thread.updated_at))
                .map(|(_, thread)| thread)
        })
}

fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn relation_score(lhs: &Path, rhs: &Path) -> Option<usize> {
    if is_component_prefix(lhs, rhs) || is_component_prefix(rhs, lhs) {
        Some(common_component_count(lhs, rhs))
    } else {
        None
    }
}

fn common_component_count(lhs: &Path, rhs: &Path) -> usize {
    lhs.components()
        .zip(rhs.components())
        .take_while(|(left, right)| left == right)
        .count()
}

fn is_component_prefix(prefix: &Path, candidate: &Path) -> bool {
    let prefix_components = prefix.components().collect::<Vec<_>>();
    let candidate_components = candidate.components().collect::<Vec<_>>();
    prefix_components.len() <= candidate_components.len()
        && prefix_components
            .iter()
            .zip(candidate_components.iter())
            .all(|(left, right)| left == right)
}

fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

#[cfg(test)]
mod tests {
    use super::{
        mutate_thread_archive_state_at, read_thread_for_id, read_threads_from_db,
        select_latest_thread_for_cwd, ThreadArchiveFilter,
    };
    use rusqlite::Connection;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_stamp() -> u128 {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            + NEXT_ID.fetch_add(1, Ordering::Relaxed) as u128
    }

    fn temp_db_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pad-codex-state-{}.sqlite", temp_stamp()))
    }

    fn temp_codex_home() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pad-codex-home-{}", temp_stamp()))
    }

    fn sample_rollout_name(thread_id: &str) -> String {
        format!("rollout-2026-03-27T14-05-10-{}.jsonl", thread_id)
    }

    fn write_rollout(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, "{\"type\":\"message\"}\n").unwrap();
    }

    fn cleanup_file(path: &Path) {
        fs::remove_file(path).ok();
    }

    fn cleanup_dir(path: &Path) {
        fs::remove_dir_all(path).ok();
    }

    fn create_threads_db(path: &Path) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    cwd TEXT NOT NULL,
                    updated_at INTEGER NOT NULL,
                    rollout_path TEXT NOT NULL,
                    title TEXT,
                    first_user_message TEXT,
                    source TEXT,
                    archived INTEGER NOT NULL DEFAULT 0,
                    archived_at INTEGER
                );",
            )
            .unwrap();
    }

    fn insert_thread(
        connection: &Connection,
        thread_id: &str,
        cwd: &str,
        updated_at: i64,
        rollout_path: &Path,
        archived: bool,
    ) {
        connection
            .execute(
                "INSERT INTO threads (
                    id, cwd, updated_at, rollout_path, title, first_user_message, source, archived, archived_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                (
                    thread_id,
                    cwd,
                    updated_at,
                    rollout_path.to_string_lossy().to_string(),
                    "hello",
                    "hello",
                    "cli",
                    if archived { 1_i64 } else { 0_i64 },
                    Option::<i64>::None,
                ),
            )
            .unwrap();
    }

    fn thread_rollout_path(connection: &Connection, thread_id: &str) -> String {
        connection
            .query_row(
                "SELECT rollout_path FROM threads WHERE id = ?1",
                [thread_id],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn thread_archive_state(connection: &Connection, thread_id: &str) -> (bool, Option<i64>, i64) {
        connection
            .query_row(
                "SELECT archived, archived_at, updated_at FROM threads WHERE id = ?1",
                [thread_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? != 0,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .unwrap()
    }

    #[test]
    fn loads_threads_from_state_db() {
        let path = temp_db_path();
        create_threads_db(&path);
        let rollout_path = std::env::temp_dir().join(format!("pad-rollout-{}.jsonl", temp_stamp()));
        write_rollout(&rollout_path);
        let connection = Connection::open(&path).unwrap();
        insert_thread(
            &connection,
            "thread-a",
            "/tmp/project",
            super::unix_now_ts(),
            &rollout_path,
            false,
        );

        let threads = read_threads_from_db(&path, ThreadArchiveFilter::ActiveOnly).unwrap();
        cleanup_file(&path);
        cleanup_file(&rollout_path);

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].thread_id, "thread-a");
        assert!(threads[0].updated_at > 0);
        assert!(!threads[0].archived);
    }

    #[test]
    fn old_threads_without_recent_updated_at_are_filtered_out() {
        let path = temp_db_path();
        create_threads_db(&path);
        let rollout_path =
            std::env::temp_dir().join(format!("pad-old-rollout-{}.jsonl", temp_stamp()));
        write_rollout(&rollout_path);
        let connection = Connection::open(&path).unwrap();
        insert_thread(
            &connection,
            "thread-old",
            "/tmp/project",
            42_i64,
            &rollout_path,
            false,
        );

        let stale_updated_at = super::unix_now_ts() - super::ACTIVE_THREAD_MAX_AGE_SECS - 60;
        connection
            .execute(
                "UPDATE threads SET updated_at = ?1 WHERE id = ?2",
                (stale_updated_at, "thread-old"),
            )
            .unwrap();

        let threads = read_threads_from_db(&path, ThreadArchiveFilter::ActiveOnly).unwrap();
        cleanup_file(&path);
        cleanup_file(&rollout_path);

        assert!(threads.is_empty());
    }

    #[test]
    fn archived_threads_are_loaded_without_recent_filter() {
        let path = temp_db_path();
        create_threads_db(&path);
        let rollout_path =
            std::env::temp_dir().join(format!("pad-archived-{}.jsonl", temp_stamp()));
        write_rollout(&rollout_path);
        let connection = Connection::open(&path).unwrap();
        insert_thread(
            &connection,
            "thread-archived",
            "/tmp/project",
            1_i64,
            &rollout_path,
            true,
        );

        let threads = read_threads_from_db(&path, ThreadArchiveFilter::ArchivedOnly).unwrap();
        cleanup_file(&path);
        cleanup_file(&rollout_path);

        assert_eq!(threads.len(), 1);
        assert!(threads[0].archived);
    }

    #[test]
    fn thread_for_id_reads_single_row_without_recent_filter() {
        let path = temp_db_path();
        create_threads_db(&path);
        let rollout_path =
            std::env::temp_dir().join(format!("pad-thread-id-{}.jsonl", temp_stamp()));
        write_rollout(&rollout_path);
        let connection = Connection::open(&path).unwrap();
        insert_thread(
            &connection,
            "thread-direct",
            "/tmp/project",
            1_i64,
            &rollout_path,
            false,
        );
        let stale_updated_at = super::unix_now_ts() - super::ACTIVE_THREAD_MAX_AGE_SECS - 60;
        connection
            .execute(
                "UPDATE threads SET updated_at = ?1 WHERE id = ?2",
                (stale_updated_at, "thread-direct"),
            )
            .unwrap();

        let thread = read_thread_for_id(&path, "thread-direct").unwrap();
        cleanup_file(&path);
        cleanup_file(&rollout_path);

        assert!(thread.is_some());
        assert_eq!(thread.unwrap().thread_id, "thread-direct");
    }

    #[test]
    fn prefers_exact_cwd_match_before_related_threads() {
        let threads = vec![
            super::CodexThreadRef {
                thread_id: "older-exact".into(),
                cwd: "/tmp/project".into(),
                updated_at: 100,
                rollout_path: "/tmp/a.jsonl".into(),
                title: None,
                first_user_message: None,
                source: None,
                archived: false,
            },
            super::CodexThreadRef {
                thread_id: "newer-parent".into(),
                cwd: "/tmp".into(),
                updated_at: 999,
                rollout_path: "/tmp/b.jsonl".into(),
                title: None,
                first_user_message: None,
                source: None,
                archived: false,
            },
        ];

        let selected = select_latest_thread_for_cwd(Path::new("/tmp/project"), &threads).unwrap();
        assert_eq!(selected.thread_id, "older-exact");
    }

    #[test]
    fn falls_back_to_closest_related_thread_when_exact_match_missing() {
        let threads = vec![
            super::CodexThreadRef {
                thread_id: "generic-parent".into(),
                cwd: "/tmp".into(),
                updated_at: 999,
                rollout_path: "/tmp/a.jsonl".into(),
                title: None,
                first_user_message: None,
                source: None,
                archived: false,
            },
            super::CodexThreadRef {
                thread_id: "project-parent".into(),
                cwd: "/tmp/project".into(),
                updated_at: 200,
                rollout_path: "/tmp/b.jsonl".into(),
                title: None,
                first_user_message: None,
                source: None,
                archived: false,
            },
        ];

        let selected =
            select_latest_thread_for_cwd(Path::new("/tmp/project/subdir"), &threads).unwrap();
        assert_eq!(selected.thread_id, "project-parent");
    }

    #[test]
    fn archive_thread_moves_rollout_and_updates_db() {
        let db_path = temp_db_path();
        let codex_home = temp_codex_home();
        create_threads_db(&db_path);

        let thread_id = "019d2de5-879e-7330-a83e-16ed3e454f69";
        let file_name = sample_rollout_name(thread_id);
        let source_path = codex_home
            .join("sessions")
            .join("2026")
            .join("03")
            .join("27")
            .join(&file_name);
        let target_path = codex_home.join("archived_sessions").join(&file_name);
        write_rollout(&source_path);

        let connection = Connection::open(&db_path).unwrap();
        insert_thread(
            &connection,
            thread_id,
            "/tmp/project",
            42_i64,
            &source_path,
            false,
        );

        mutate_thread_archive_state_at(&db_path, &codex_home, thread_id, true).unwrap();

        assert!(!source_path.exists());
        assert!(target_path.exists());
        assert_eq!(
            thread_rollout_path(&connection, thread_id),
            target_path.display().to_string()
        );
        let (archived, archived_at, updated_at) = thread_archive_state(&connection, thread_id);
        assert!(archived);
        assert!(archived_at.is_some());
        assert_eq!(updated_at, 42);

        cleanup_file(&db_path);
        cleanup_dir(&codex_home);
    }

    #[test]
    fn unarchive_thread_moves_rollout_back_and_updates_db() {
        let db_path = temp_db_path();
        let codex_home = temp_codex_home();
        create_threads_db(&db_path);

        let thread_id = "019d2de5-879e-7330-a83e-16ed3e454f70";
        let file_name = sample_rollout_name(thread_id);
        let source_path = codex_home.join("archived_sessions").join(&file_name);
        let target_path = codex_home
            .join("sessions")
            .join("2026")
            .join("03")
            .join("27")
            .join(&file_name);
        write_rollout(&source_path);

        let connection = Connection::open(&db_path).unwrap();
        insert_thread(
            &connection,
            thread_id,
            "/tmp/project",
            42_i64,
            &source_path,
            true,
        );
        connection
            .execute(
                "UPDATE threads SET archived_at = ?1 WHERE id = ?2",
                (99_i64, thread_id),
            )
            .unwrap();

        mutate_thread_archive_state_at(&db_path, &codex_home, thread_id, false).unwrap();

        assert!(!source_path.exists());
        assert!(target_path.exists());
        assert_eq!(
            thread_rollout_path(&connection, thread_id),
            target_path.display().to_string()
        );
        let (archived, archived_at, updated_at) = thread_archive_state(&connection, thread_id);
        assert!(!archived);
        assert!(archived_at.is_none());
        assert!(updated_at >= 42);

        cleanup_file(&db_path);
        cleanup_dir(&codex_home);
    }
}
