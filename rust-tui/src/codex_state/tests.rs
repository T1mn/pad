mod archive;
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
mod migration;
mod query;
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
mod support;
