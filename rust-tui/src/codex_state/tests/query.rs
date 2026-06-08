use super::super::query::{read_thread_for_id, read_threads_from_db};
use super::super::ThreadArchiveFilter;
use super::support::{
    cleanup_file, create_threads_db, insert_thread, temp_db_path, temp_rollout_path, write_rollout,
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

    let stale_updated_at =
        super::super::util::unix_now_ts() - super::super::model::ACTIVE_THREAD_MAX_AGE_SECS - 60;
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
    let stale_updated_at =
        super::super::util::unix_now_ts() - super::super::model::ACTIVE_THREAD_MAX_AGE_SECS - 60;
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
