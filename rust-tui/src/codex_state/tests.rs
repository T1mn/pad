mod archive {
    use super::super::archive::mutate_thread_archive_state_at;
    use super::support::{
        cleanup_dir, cleanup_file, create_threads_db, insert_thread, sample_rollout_name,
        temp_codex_home, temp_db_path, thread_archive_state, thread_rollout_path, write_rollout,
    };
    use rusqlite::Connection;

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

    #[cfg(unix)]
    #[test]
    fn archive_thread_accepts_rollout_through_symlinked_sessions_dir() {
        let db_path = temp_db_path();
        let codex_home = temp_codex_home();
        let pad_home = temp_codex_home();
        create_threads_db(&db_path);

        let thread_id = "019d2de5-879e-7330-a83e-16ed3e454f71";
        let file_name = sample_rollout_name(thread_id);
        let canonical_source_path = codex_home
            .join("sessions")
            .join("2026")
            .join("03")
            .join("27")
            .join(&file_name);
        write_rollout(&canonical_source_path);

        std::fs::create_dir_all(&pad_home).unwrap();
        std::os::unix::fs::symlink(codex_home.join("sessions"), pad_home.join("sessions")).unwrap();
        let symlink_source_path = pad_home
            .join("sessions")
            .join("2026")
            .join("03")
            .join("27")
            .join(&file_name);
        let target_path = codex_home.join("archived_sessions").join(&file_name);

        let connection = Connection::open(&db_path).unwrap();
        insert_thread(
            &connection,
            thread_id,
            "/tmp/project",
            42_i64,
            &symlink_source_path,
            false,
        );

        mutate_thread_archive_state_at(&db_path, &codex_home, thread_id, true).unwrap();

        assert!(!canonical_source_path.exists());
        assert!(target_path.exists());
        assert_eq!(
            thread_rollout_path(&connection, thread_id),
            target_path.display().to_string()
        );

        cleanup_file(&db_path);
        cleanup_dir(&codex_home);
        cleanup_dir(&pad_home);
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
mod archive_compressed {
    use super::super::archive::mutate_thread_archive_state_at;
    use super::support::{
        cleanup_dir, cleanup_file, create_threads_db, insert_thread, sample_rollout_name,
        temp_codex_home, temp_db_path, thread_rollout_path, write_rollout,
    };
    use rusqlite::Connection;
    use std::path::{Path, PathBuf};

    fn compressed_sibling(path: &Path) -> PathBuf {
        let mut compressed = path.as_os_str().to_os_string();
        compressed.push(".zst");
        compressed.into()
    }

    #[test]
    fn archive_thread_resolves_compressed_rollout_sibling() {
        let db_path = temp_db_path();
        let codex_home = temp_codex_home();
        create_threads_db(&db_path);
        let thread_id = "019d2de5-879e-7330-a83e-16ed3e454f72";
        let file_name = sample_rollout_name(thread_id);
        let canonical = codex_home.join("sessions/2026/03/27").join(&file_name);
        let source = compressed_sibling(&canonical);
        let target = compressed_sibling(&codex_home.join("archived_sessions").join(&file_name));
        write_rollout(&source);
        let connection = Connection::open(&db_path).unwrap();
        insert_thread(
            &connection,
            thread_id,
            "/tmp/project",
            42,
            &canonical,
            false,
        );

        mutate_thread_archive_state_at(&db_path, &codex_home, thread_id, true).unwrap();
        assert!(!source.exists());
        assert!(target.exists());
        assert_eq!(
            thread_rollout_path(&connection, thread_id),
            target.display().to_string()
        );
        cleanup_file(&db_path);
        cleanup_dir(&codex_home);
    }

    #[test]
    fn unarchive_thread_resolves_compressed_rollout_sibling() {
        let db_path = temp_db_path();
        let codex_home = temp_codex_home();
        create_threads_db(&db_path);
        let thread_id = "019d2de5-879e-7330-a83e-16ed3e454f73";
        let file_name = sample_rollout_name(thread_id);
        let canonical = codex_home.join("archived_sessions").join(&file_name);
        let source = compressed_sibling(&canonical);
        let target = compressed_sibling(&codex_home.join("sessions/2026/03/27").join(&file_name));
        write_rollout(&source);
        let connection = Connection::open(&db_path).unwrap();
        insert_thread(&connection, thread_id, "/tmp/project", 42, &canonical, true);
        connection
            .execute(
                "UPDATE threads SET archived_at = ?1 WHERE id = ?2",
                (99_i64, thread_id),
            )
            .unwrap();

        mutate_thread_archive_state_at(&db_path, &codex_home, thread_id, false).unwrap();
        assert!(!source.exists());
        assert!(target.exists());
        assert_eq!(
            thread_rollout_path(&connection, thread_id),
            target.display().to_string()
        );
        cleanup_file(&db_path);
        cleanup_dir(&codex_home);
    }
}
mod migration {
    use super::super::migration::normalize_pad_codex_home_rollout_paths_at;
    use super::support::{
        cleanup_dir, cleanup_file, create_threads_db, insert_thread, sample_rollout_name,
        temp_codex_home, temp_db_path, thread_rollout_path,
    };
    use rusqlite::Connection;

    #[test]
    fn normalize_pad_codex_home_rollout_paths_rewrites_shared_prefixes() {
        let db_path = temp_db_path();
        let canonical_home = temp_codex_home();
        let pad_home = temp_codex_home();
        create_threads_db(&db_path);

        let active_id = "019d2de5-879e-7330-a83e-16ed3e454f72";
        let archived_id = "019d2de5-879e-7330-a83e-16ed3e454f73";
        let native_id = "019d2de5-879e-7330-a83e-16ed3e454f74";
        let active_path = pad_home
            .join("sessions")
            .join("2026")
            .join("03")
            .join("27")
            .join(sample_rollout_name(active_id));
        let archived_path = pad_home
            .join("archived_sessions")
            .join(sample_rollout_name(archived_id));
        let native_path = canonical_home
            .join("sessions")
            .join("2026")
            .join("03")
            .join("27")
            .join(sample_rollout_name(native_id));

        let connection = Connection::open(&db_path).unwrap();
        insert_thread(
            &connection,
            active_id,
            "/tmp/project",
            42,
            &active_path,
            false,
        );
        insert_thread(
            &connection,
            archived_id,
            "/tmp/project",
            43,
            &archived_path,
            true,
        );
        insert_thread(
            &connection,
            native_id,
            "/tmp/project",
            44,
            &native_path,
            false,
        );

        let updated =
            normalize_pad_codex_home_rollout_paths_at(&db_path, &pad_home, &canonical_home)
                .unwrap();

        assert_eq!(updated, 2);
        assert_eq!(
            thread_rollout_path(&connection, active_id),
            canonical_home
                .join("sessions")
                .join("2026")
                .join("03")
                .join("27")
                .join(sample_rollout_name(active_id))
                .display()
                .to_string()
        );
        assert_eq!(
            thread_rollout_path(&connection, archived_id),
            canonical_home
                .join("archived_sessions")
                .join(sample_rollout_name(archived_id))
                .display()
                .to_string()
        );
        assert_eq!(
            thread_rollout_path(&connection, native_id),
            native_path.display().to_string()
        );

        cleanup_file(&db_path);
        cleanup_dir(&canonical_home);
        cleanup_dir(&pad_home);
    }

    #[test]
    fn normalize_pad_codex_home_rollout_paths_handles_non_ascii_home() {
        let db_path = temp_db_path();
        let unicode_root = temp_codex_home().join("用户");
        let pad_home = unicode_root.join("pad-codex-home");
        let canonical_home = unicode_root.join("canonical-codex-home");
        create_threads_db(&db_path);

        let thread_id = "019d2de5-879e-7330-a83e-16ed3e454f75";
        let rollout_path = pad_home
            .join("sessions")
            .join("2026")
            .join("03")
            .join("27")
            .join(sample_rollout_name(thread_id));
        let connection = Connection::open(&db_path).unwrap();
        insert_thread(
            &connection,
            thread_id,
            "/tmp/project",
            45,
            &rollout_path,
            false,
        );

        let updated =
            normalize_pad_codex_home_rollout_paths_at(&db_path, &pad_home, &canonical_home)
                .unwrap();

        assert_eq!(updated, 1);
        assert_eq!(
            thread_rollout_path(&connection, thread_id),
            canonical_home
                .join("sessions")
                .join("2026")
                .join("03")
                .join("27")
                .join(sample_rollout_name(thread_id))
                .display()
                .to_string()
        );

        cleanup_file(&db_path);
    }
}
mod query {
    use super::super::query::{read_thread_for_id, read_threads_from_db};
    use super::super::ThreadArchiveFilter;
    use super::support::{
        cleanup_file, create_threads_db, insert_thread, temp_db_path, temp_rollout_path,
        write_rollout,
    };
    use rusqlite::Connection;

    #[test]
    fn loads_threads_from_state_db() {
        let path = temp_db_path();
        create_threads_db(&path);
        let rollout_path = temp_rollout_path("new");
        write_rollout(&rollout_path);
        let connection = Connection::open(&path).unwrap();
        insert_thread(
            &connection,
            "thread-a",
            "/tmp/project",
            super::super::util::unix_now_ts(),
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
        let rollout_path = temp_rollout_path("old");
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

        let stale_updated_at = super::super::util::unix_now_ts()
            - super::super::model::ACTIVE_THREAD_MAX_AGE_SECS
            - 60;
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
        let rollout_path = temp_rollout_path("archived");
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
        let rollout_path = temp_rollout_path("thread-id");
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
        let stale_updated_at = super::super::util::unix_now_ts()
            - super::super::model::ACTIVE_THREAD_MAX_AGE_SECS
            - 60;
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
}
mod selection {
    use super::super::pathing::{is_component_prefix, select_latest_thread_for_cwd};
    use std::path::Path;

    #[test]
    fn component_prefix_does_not_match_sibling_paths() {
        assert!(is_component_prefix(
            Path::new("/tmp/project"),
            Path::new("/tmp/project/subdir")
        ));
        assert!(!is_component_prefix(
            Path::new("/tmp/project"),
            Path::new("/tmp/project-old")
        ));
    }

    #[test]
    fn prefers_exact_cwd_match_before_related_threads() {
        let threads = vec![
            super::super::CodexThreadRef {
                thread_id: "older-exact".into(),
                cwd: "/tmp/project".into(),
                updated_at: 100,
                rollout_path: "/tmp/a.jsonl".into(),
                title: None,
                first_user_message: None,
                source: None,
                archived: false,
            },
            super::super::CodexThreadRef {
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
            super::super::CodexThreadRef {
                thread_id: "generic-parent".into(),
                cwd: "/tmp".into(),
                updated_at: 999,
                rollout_path: "/tmp/a.jsonl".into(),
                title: None,
                first_user_message: None,
                source: None,
                archived: false,
            },
            super::super::CodexThreadRef {
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
}
mod support {
    use rusqlite::Connection;
    use std::fs;
    use std::path::Path;

    pub(super) fn temp_db_path() -> std::path::PathBuf {
        crate::test_support::temp_path("pad-codex-state", "db")
    }

    pub(super) fn temp_codex_home() -> std::path::PathBuf {
        crate::test_support::temp_path("pad-codex-home", "root")
    }

    pub(super) fn temp_rollout_path(name: &str) -> std::path::PathBuf {
        crate::test_support::temp_path("pad-codex-rollout", name)
    }

    pub(super) fn sample_rollout_name(thread_id: &str) -> String {
        format!("rollout-2026-03-27T14-05-10-{}.jsonl", thread_id)
    }

    pub(super) fn write_rollout(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, "{\"type\":\"message\"}\n").unwrap();
    }

    pub(super) fn cleanup_file(path: &Path) {
        fs::remove_file(path).ok();
    }

    pub(super) fn cleanup_dir(path: &Path) {
        fs::remove_dir_all(path).ok();
    }

    pub(super) fn create_threads_db(path: &Path) {
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

    pub(super) fn insert_thread(
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

    pub(super) fn thread_rollout_path(connection: &Connection, thread_id: &str) -> String {
        connection
            .query_row(
                "SELECT rollout_path FROM threads WHERE id = ?1",
                [thread_id],
                |row| row.get(0),
            )
            .unwrap()
    }

    pub(super) fn thread_archive_state(
        connection: &Connection,
        thread_id: &str,
    ) -> (bool, Option<i64>, i64) {
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
}
