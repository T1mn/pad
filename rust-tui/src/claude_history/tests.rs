use super::api::{load_threads_at, thread_for_id_at};
use super::db::{mutate_thread_archive_state_at, query_threads_at, upsert_hook_session_at};
use super::model::ThreadArchiveFilter;
use super::parse::parse_claude_thread_file;
use super::scan::{discover_thread_files, sync_index_at};
use super::util::normalize_path;
use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("pad-claude-history-{}-{}", name, stamp));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn temp_db(name: &str) -> std::path::PathBuf {
    temp_dir(name).join("claude.sqlite")
}

fn write_thread(file: &Path, session_id: &str, cwd: &str, title: &str, assistant_ts: &str) {
    fs::write(
        file,
        format!(
            concat!(
                "{{\"type\":\"user\",\"sessionId\":\"{}\",\"cwd\":\"{}\",\"message\":{{\"role\":\"user\",\"content\":\"{}\"}}}}\n",
                "{{\"type\":\"assistant\",\"sessionId\":\"{}\",\"cwd\":\"{}\",\"message\":{{\"role\":\"assistant\",\"content\":\"ok\"}},\"timestamp\":\"{}\"}}\n"
            ),
            session_id, cwd, title, session_id, cwd, assistant_ts
        ),
    )
    .unwrap();
}

#[test]
fn parse_claude_thread_file_extracts_session_cwd_and_title() {
    let dir = temp_dir("single");
    let file = dir.join("sample.jsonl");
    write_thread(
        &file,
        "abc",
        "/tmp/project",
        "first prompt",
        "2099-03-10T05:41:54.280Z",
    );

    let parsed = parse_claude_thread_file(&file).unwrap().unwrap();
    fs::remove_dir_all(&dir).ok();

    assert_eq!(parsed.session_id, "abc");
    assert_eq!(parsed.cwd, Path::new("/tmp/project"));
    assert_eq!(parsed.title.as_deref(), Some("first prompt"));
}

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

    sync_index_at(&root, &db).unwrap();
    let first = load_threads_at(&root, &db, ThreadArchiveFilter::ActiveOnly).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].session_id, "a");

    sleep(Duration::from_millis(1100));
    write_thread(
        &file_b,
        "b",
        "/tmp/project-b",
        "prompt b",
        "2099-03-10T05:41:54.280Z",
    );
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

#[test]
fn progress_only_stub_file_is_filtered_out() {
    let dir = temp_dir("progress-only");
    let file = dir.join("stub.jsonl");
    fs::write(
        &file,
        "{\"type\":\"progress\",\"sessionId\":\"stub\",\"cwd\":\"/tmp/project\",\"data\":{\"type\":\"hook_progress\"}}\n",
    )
    .unwrap();

    let parsed = parse_claude_thread_file(&file).unwrap();
    fs::remove_dir_all(&dir).ok();

    assert!(parsed.is_none());
}

#[test]
fn sidechain_file_is_filtered_out() {
    let dir = temp_dir("sidechain");
    let file = dir.join("agent-sidechain.jsonl");
    fs::write(
        &file,
        concat!(
            "{\"type\":\"user\",\"isSidechain\":true,\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"subagent\"}}\n",
            "{\"type\":\"assistant\",\"isSidechain\":true,\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"assistant\",\"content\":\"ok\"}}\n"
        ),
    )
    .unwrap();

    let parsed = parse_claude_thread_file(&file).unwrap();
    fs::remove_dir_all(&dir).ok();

    assert!(parsed.is_none());
}

#[test]
fn local_command_scaffold_is_not_used_as_title() {
    let dir = temp_dir("local-command");
    let file = dir.join("main.jsonl");
    fs::write(
        &file,
        concat!(
            "{\"type\":\"user\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"<local-command-caveat>do not use</local-command-caveat>\"}}\n",
            "{\"type\":\"user\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"<command-name>/clear</command-name><command-message>clear</command-message>\"}}\n",
            "{\"type\":\"user\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"真实用户问题\"}}\n",
            "{\"type\":\"assistant\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"assistant\",\"content\":\"ok\"}}\n"
        ),
    )
    .unwrap();

    let parsed = parse_claude_thread_file(&file).unwrap().unwrap();
    fs::remove_dir_all(&dir).ok();

    assert_eq!(parsed.title.as_deref(), Some("真实用户问题"));
}

#[test]
fn read_threads_ignores_subagents_directory() {
    let root = temp_dir("subagents-dir");
    let main_file = root.join("main.jsonl");
    write_thread(
        &main_file,
        "main",
        "/tmp/project",
        "main prompt",
        "2099-03-10T05:41:54.280Z",
    );

    let sub_dir = root.join("subagents");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(
        sub_dir.join("agent-a79dd02e.jsonl"),
        concat!(
            "{\"type\":\"user\",\"isSidechain\":true,\"sessionId\":\"main\",\"cwd\":\"/tmp/worktree\",\"message\":{\"role\":\"user\",\"content\":\"sidechain\"}}\n",
            "{\"type\":\"assistant\",\"isSidechain\":true,\"sessionId\":\"main\",\"cwd\":\"/tmp/worktree\",\"message\":{\"role\":\"assistant\",\"content\":\"ok\"}}\n"
        ),
    )
    .unwrap();

    let result = discover_thread_files(&root).unwrap();
    fs::remove_dir_all(&root).ok();

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].file_name().and_then(|name| name.to_str()),
        Some("main.jsonl")
    );
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
