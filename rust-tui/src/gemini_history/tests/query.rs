use super::super::{sync_index_at, threads_for_cwd_at};
use super::support::{sample_session_json, temp_db, temp_root, write_project_session};
use std::fs;
use std::path::Path;

#[test]
fn threads_for_cwd_uses_project_root() {
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

    let threads = threads_for_cwd_at(&root, &db, Path::new("/Users/tim/example/project")).unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].session_id, "session-2");
}

#[test]
fn normalized_project_root_matches_cwd_query() {
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
