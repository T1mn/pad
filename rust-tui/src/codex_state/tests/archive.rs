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
