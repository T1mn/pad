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
        normalize_pad_codex_home_rollout_paths_at(&db_path, &pad_home, &canonical_home).unwrap();

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
        normalize_pad_codex_home_rollout_paths_at(&db_path, &pad_home, &canonical_home).unwrap();

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
