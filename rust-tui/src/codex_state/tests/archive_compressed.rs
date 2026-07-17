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
