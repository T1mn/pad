use super::super::parse::parse_claude_thread_file;
use super::super::scan::discover_thread_files;
use super::support::{temp_dir, write_thread};
use std::fs;
use std::path::Path;

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
fn local_command_scaffold_filter_is_case_insensitive() {
    let dir = temp_dir("local-command-case");
    let file = dir.join("main.jsonl");
    fs::write(
        &file,
        concat!(
            "{\"type\":\"user\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"<COMMAND-NAME>/clear</COMMAND-NAME>\"}}\n",
            "{\"type\":\"user\",\"sessionId\":\"main\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"real prompt\"}}\n"
        ),
    )
    .unwrap();

    let parsed = parse_claude_thread_file(&file).unwrap().unwrap();
    fs::remove_dir_all(&dir).ok();

    assert_eq!(parsed.title.as_deref(), Some("real prompt"));
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
