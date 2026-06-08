use super::super::api::thread_for_id_at;
use super::super::db::{mutate_thread_archive_state_at, query_threads_at, upsert_hook_session_at};
use super::super::model::ThreadArchiveFilter;
use super::super::scan::sync_index_at;
use super::super::util::normalize_path;
use super::support::{temp_db, temp_dir, write_thread};
use std::fs;

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
