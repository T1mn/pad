use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn temp_root(name: &str) -> PathBuf {
    let root = crate::test_support::temp_path("pad-gemini-root", name);
    fs::create_dir_all(&root).unwrap();
    root
}

pub(super) fn temp_db(name: &str) -> PathBuf {
    crate::test_support::temp_path("pad-gemini-db", name).with_extension("sqlite")
}

pub(super) fn write_project_session(
    root: &Path,
    alias: &str,
    session_name: &str,
    json: &str,
) -> PathBuf {
    let project_dir = root.join(alias);
    let chats_dir = project_dir.join("chats");
    fs::create_dir_all(&chats_dir).unwrap();
    fs::write(
        project_dir.join(".project_root"),
        "/Users/tim/example/project\n",
    )
    .unwrap();
    let path = chats_dir.join(session_name);
    fs::write(&path, json).unwrap();
    path
}

pub(super) fn sample_session_json(
    session_id: &str,
    kind: &str,
    summary: Option<&str>,
    last_updated: &str,
    user_text: &str,
    assistant_text: &str,
) -> String {
    let summary_json = summary
        .map(|s| format!(r#","summary":"{}""#, s))
        .unwrap_or_default();
    format!(
        r#"{{
  "sessionId": "{session_id}",
  "projectHash": "hash",
  "kind": "{kind}",
  "startTime": "2026-03-28T04:00:00.000Z",
  "lastUpdated": "{last_updated}",
  "messages": [
    {{
      "id": "u1",
      "timestamp": "2026-03-28T04:00:01.000Z",
      "type": "user",
      "content": [{{"text": "{user_text}"}}]
    }},
    {{
      "id": "a1",
      "timestamp": "2026-03-28T04:00:02.000Z",
      "type": "gemini",
      "content": "{assistant_text}",
      "tokens": {{"total": 1}}
    }}
  ]{summary_json}
}}"#
    )
}
