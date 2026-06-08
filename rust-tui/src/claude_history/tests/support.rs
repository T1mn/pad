use std::fs;
use std::path::Path;

pub(super) fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = crate::test_support::temp_path("pad-claude-history", name);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) fn temp_db(name: &str) -> std::path::PathBuf {
    temp_dir(name).join("claude.sqlite")
}

pub(super) fn write_thread(
    file: &Path,
    session_id: &str,
    cwd: &str,
    title: &str,
    assistant_ts: &str,
) {
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
