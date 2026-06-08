use super::super::{
    all_archived_threads_at, all_threads_at, archive_thread_at, sync_index_at, thread_for_id_at,
};
use super::support::{sample_session_json, temp_db, temp_root, write_project_session};
use std::fs;

#[test]
fn main_snapshot_wins_over_subagent_and_archive_is_local() {
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

#[test]
fn archive_by_session_id_updates_all_matching_rows() {
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
