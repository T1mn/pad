#[cfg(test)]
pub(crate) mod archive {
    use super::super::{
        all_archived_threads_at, all_threads_at, archive_thread_at, sync_index_at, thread_for_id_at,
    };
    use super::support::{sample_session_json, temp_db, temp_root, write_project_session};
    use std::fs;

    pub(crate) fn main_snapshot_wins_over_subagent_and_archive_is_local() {
        let root = temp_root("main-snapshot");
        let db = temp_db("main-snapshot");
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

    pub(crate) fn archive_by_session_id_updates_all_matching_rows() {
        let root = temp_root("archive-shared-session");
        let db = temp_db("archive-shared-session");
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
}
#[cfg(test)]
pub(crate) mod query {
    use super::super::{sync_index_at, threads_for_cwd_at};
    use super::support::{sample_session_json, temp_db, temp_root, write_project_session};
    use std::fs;
    use std::path::Path;

    pub(crate) fn threads_for_cwd_uses_project_root() {
        let root = temp_root("cwd-project-root");
        let db = temp_db("cwd-project-root");
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

    pub(crate) fn normalized_project_root_matches_cwd_query() {
        let root = temp_root("normalized-project-root");
        let db = temp_db("normalized-project-root");
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
}
#[cfg(test)]
pub(crate) mod scan {
    use super::super::{all_threads_at, sync_index_at};
    use super::support::{sample_session_json, temp_db, temp_root, write_project_session};
    use std::fs;

    pub(crate) fn invalid_snapshot_does_not_break_sync() {
        let root = temp_root("invalid-snapshot");
        let db = temp_db("invalid-snapshot");
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

    pub(crate) fn scan_joins_nested_message_parts_without_empty_entries() {
        let root = temp_root("joined-message-parts");
        let db = temp_db("joined-message-parts");
        write_project_session(
            &root,
            "parts",
            "session-main.json",
            r#"{
              "sessionId": "session-parts",
              "projectHash": "hash",
              "kind": "main",
              "startTime": "2026-03-28T04:00:00.000Z",
              "lastUpdated": "2026-03-28T04:00:02.000Z",
              "messages": [
                {
                  "type": "user",
                  "content": [
                    {"text": "hello"},
                    {"text": "   "},
                    {"content": {"text": "world"}}
                  ]
                },
                {
                  "type": "gemini",
                  "content": [
                    {"text": "answer"},
                    {"parts": [{"text": "more"}]}
                  ]
                }
              ]
            }"#,
        );

        sync_index_at(&root, &db).unwrap();
        let threads = all_threads_at(&root, &db).unwrap();

        assert_eq!(threads.len(), 1);
        assert_eq!(
            threads[0].first_user_message.as_deref(),
            Some("hello\nworld")
        );
        assert_eq!(
            threads[0].last_assistant_message.as_deref(),
            Some("answer\nmore")
        );
    }

    pub(crate) fn indexed_rows_survive_when_source_snapshots_disappear() {
        let root = temp_root("source-snapshots-disappear");
        let db = temp_db("source-snapshots-disappear");
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
#[cfg(test)]
mod support {
    use std::fs;
    use std::path::{Path, PathBuf};

    pub(super) fn temp_root(name: &str) -> PathBuf {
        let root = crate::test_support::temp_path("pad-gemini-root", name);
        fs::create_dir_all(&root).unwrap();
        root
    }

    pub(super) fn temp_db(name: &str) -> PathBuf {
        crate::test_support::temp_path("pad-gemini-db", name).with_extension("sqlite")
    }

    pub(super) fn write_project_session(
        root: &Path,
        alias: &str,
        session_name: &str,
        json: &str,
    ) -> PathBuf {
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

    pub(super) fn sample_session_json(
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
}
