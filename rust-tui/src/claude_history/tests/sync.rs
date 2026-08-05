use super::super::api::{load_threads_at, thread_for_id_at};
use super::super::db::query_threads_at;
use super::super::model::ThreadArchiveFilter;
use super::super::scan::sync_index_at;
use super::support::{temp_db, temp_dir, write_thread};
use std::fs::{self, FileTimes, OpenOptions};
use std::time::{Duration, UNIX_EPOCH};

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
    set_modified_secs(&file_a, 2_000_000_000);

    sync_index_at(&root, &db).unwrap();
    let first = load_threads_at(&root, &db, ThreadArchiveFilter::ActiveOnly).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].session_id, "a");

    write_thread(
        &file_b,
        "b",
        "/tmp/project-b",
        "prompt b",
        "2099-03-10T05:41:54.280Z",
    );
    set_modified_secs(&file_b, 2_000_000_001);
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

fn set_modified_secs(path: &std::path::Path, seconds: u64) {
    OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(seconds)))
        .unwrap();
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
