#[cfg(test)]
mod cases {
    use super::super::{
        all_archived_threads_at, all_threads_at, archive_thread_at, sync_index_at,
        thread_for_id_at, threads_for_cwd_at,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_stamp() -> u128 {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            + COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as u128
    }

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!("pad-gemini-root-{}", temp_stamp()))
    }

    fn temp_db() -> PathBuf {
        std::env::temp_dir().join(format!("pad-gemini-db-{}.sqlite", temp_stamp()))
    }

    fn write_project_session(root: &Path, alias: &str, session_name: &str, json: &str) -> PathBuf {
        let project_dir = root.join(alias);
        let chats_dir = project_dir.join("chats");
        fs::create_dir_all(&chats_dir).unwrap();
        fs::write(
            project_dir.join(".project_root"),
            "/Users/tim/example/project\n",
        )
        .unwrap();
        let path = chats_dir.join(session_name);
        fs::write(&path, json).unwrap();
        path
    }

    fn sample_session_json(
        session_id: &str,
        kind: &str,
        summary: Option<&str>,
        last_updated: &str,
        user_text: &str,
        assistant_text: &str,
    ) -> String {
        let summary_json = summary
            .map(|s| format!(r#","summary":"{}""#, s))
            .unwrap_or_default();
        format!(
            r#"{{
  "sessionId": "{session_id}",
  "projectHash": "hash",
  "kind": "{kind}",
  "startTime": "2026-03-28T04:00:00.000Z",
  "lastUpdated": "{last_updated}",
  "messages": [
    {{
      "id": "u1",
      "timestamp": "2026-03-28T04:00:01.000Z",
      "type": "user",
      "content": [{{"text": "{user_text}"}}]
    }},
    {{
      "id": "a1",
      "timestamp": "2026-03-28T04:00:02.000Z",
      "type": "gemini",
      "content": "{assistant_text}",
      "tokens": {{"total": 1}}
    }}
  ]{summary_json}
}}"#
        )
    }

    #[test]
    fn main_snapshot_wins_over_subagent_and_archive_is_local() {
        let root = temp_root();
        let db = temp_db();
        let session_main = sample_session_json(
            "session-1",
            "main",
            Some("Main summary"),
            "2026-03-28T04:14:54.080Z",
            "hello main",
            "assistant main",
        );
        let session_sub = sample_session_json(
            "session-1",
            "subagent",
            None,
            "2026-03-28T05:14:54.080Z",
            "hello subagent",
            "assistant subagent",
        );
        write_project_session(&root, "rust-tui", "session-main.json", &session_main);
        write_project_session(&root, "rust-tui", "session-sub.json", &session_sub);

        sync_index_at(&root, &db).unwrap();
        let threads = all_threads_at(&root, &db).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].session_id, "session-1");
        assert_eq!(threads[0].title.as_deref(), Some("Main summary"));
        assert_eq!(threads[0].subtitle.as_deref(), Some("hello main"));
        assert!(threads[0].has_subagent);

        archive_thread_at(&root, &db, "session-1", true).unwrap();
        assert!(all_threads_at(&root, &db).unwrap().is_empty());
        let archived = all_archived_threads_at(&root, &db).unwrap();
        assert_eq!(archived.len(), 1);
        assert!(archived[0].archived);

        let direct = thread_for_id_at(&root, &db, "session-1").unwrap();
        assert!(direct.is_some());
        assert!(direct.unwrap().archived);

        sync_index_at(&root, &db).unwrap();
        let archived_after_rescan = all_archived_threads_at(&root, &db).unwrap();
        assert_eq!(archived_after_rescan.len(), 1);
        assert!(archived_after_rescan[0].archived);

        archive_thread_at(&root, &db, "session-1", false).unwrap();
        assert_eq!(all_threads_at(&root, &db).unwrap().len(), 1);
    }

    #[test]
    fn threads_for_cwd_uses_project_root() {
        let root = temp_root();
        let db = temp_db();
        let json = sample_session_json(
            "session-2",
            "main",
            None,
            "2026-03-28T06:14:54.080Z",
            "prompt",
            "answer",
        );
        write_project_session(&root, "rust-tui", "session-main.json", &json);
        sync_index_at(&root, &db).unwrap();

        let threads =
            threads_for_cwd_at(&root, &db, Path::new("/Users/tim/example/project")).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].session_id, "session-2");
    }

    #[test]
    fn invalid_snapshot_does_not_break_sync() {
        let root = temp_root();
        let db = temp_db();
        let valid = sample_session_json(
            "session-valid",
            "main",
            Some("Valid summary"),
            "2026-03-28T06:14:54.080Z",
            "prompt",
            "answer",
        );
        write_project_session(&root, "valid", "session-valid.json", &valid);
        write_project_session(&root, "broken", "session-broken.json", "{not-json");

        sync_index_at(&root, &db).unwrap();
        let threads = all_threads_at(&root, &db).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].session_id, "session-valid");
    }

    #[test]
    fn archive_by_session_id_updates_all_matching_rows() {
        let root = temp_root();
        let db = temp_db();
        let session_a = sample_session_json(
            "shared-session",
            "main",
            Some("Summary A"),
            "2026-03-28T06:14:54.080Z",
            "prompt a",
            "answer a",
        );
        let session_b = sample_session_json(
            "shared-session",
            "main",
            Some("Summary B"),
            "2026-03-28T07:14:54.080Z",
            "prompt b",
            "answer b",
        );

        let project_a = root.join("project-a");
        fs::create_dir_all(root.join("resolved-a")).unwrap();
        fs::create_dir_all(project_a.join("chats")).unwrap();
        fs::write(
            project_a.join(".project_root"),
            root.join("resolved-a").display().to_string(),
        )
        .unwrap();
        fs::write(
            project_a.join("chats").join("session-main-a.json"),
            session_a,
        )
        .unwrap();

        let project_b = root.join("project-b");
        fs::create_dir_all(root.join("resolved-b")).unwrap();
        fs::create_dir_all(project_b.join("chats")).unwrap();
        fs::write(
            project_b.join(".project_root"),
            root.join("resolved-b").display().to_string(),
        )
        .unwrap();
        fs::write(
            project_b.join("chats").join("session-main-b.json"),
            session_b,
        )
        .unwrap();

        sync_index_at(&root, &db).unwrap();
        assert_eq!(all_threads_at(&root, &db).unwrap().len(), 2);

        archive_thread_at(&root, &db, "shared-session", true).unwrap();
        assert!(all_threads_at(&root, &db).unwrap().is_empty());
        assert_eq!(all_archived_threads_at(&root, &db).unwrap().len(), 2);
    }

    #[test]
    fn normalized_project_root_matches_cwd_query() {
        let root = temp_root();
        let db = temp_db();
        let project_dir = root.join("rust-tui");
        let chats_dir = project_dir.join("chats");
        let real_root = root.join("workspace").join("real-project");
        fs::create_dir_all(&chats_dir).unwrap();
        fs::create_dir_all(&real_root).unwrap();
        fs::write(
            project_dir.join(".project_root"),
            root.join("workspace")
                .join("real-project")
                .join("..")
                .join("real-project")
                .display()
                .to_string(),
        )
        .unwrap();
        fs::write(
            chats_dir.join("session-main.json"),
            sample_session_json(
                "session-3",
                "main",
                None,
                "2026-03-28T08:14:54.080Z",
                "prompt",
                "answer",
            ),
        )
        .unwrap();

        sync_index_at(&root, &db).unwrap();
        let threads = threads_for_cwd_at(&root, &db, &real_root).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(
            fs::canonicalize(&threads[0].cwd).unwrap(),
            fs::canonicalize(&real_root).unwrap()
        );
    }

    #[test]
    fn indexed_rows_survive_when_source_snapshots_disappear() {
        let root = temp_root();
        let db = temp_db();
        let json = sample_session_json(
            "session-4",
            "main",
            Some("Persist me"),
            "2026-03-28T09:14:54.080Z",
            "prompt",
            "answer",
        );
        let path = write_project_session(&root, "rust-tui", "session-main.json", &json);

        sync_index_at(&root, &db).unwrap();
        assert_eq!(all_threads_at(&root, &db).unwrap().len(), 1);

        fs::remove_file(path).unwrap();
        sync_index_at(&root, &db).unwrap();
        let threads = all_threads_at(&root, &db).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].session_id, "session-4");
    }
}
