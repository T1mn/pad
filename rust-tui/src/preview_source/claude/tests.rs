use super::parse_transcript;
use crate::preview_source::SessionReadMode;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_jsonl_path(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("pad-preview-{}-{}.jsonl", name, stamp))
}

#[test]
fn parse_claude_transcript_skips_meta_thinking_and_tools() {
    let path = temp_jsonl_path("claude");
    fs::write(
        &path,
        concat!(
            "{\"type\":\"user\",\"isMeta\":true,\"message\":{\"role\":\"user\",\"content\":\"skip meta\"}}\n",
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"<command-name>/clear</command-name>\"}}\n",
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"real user\"}}\n",
            "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"skip\"},{\"type\":\"text\",\"text\":\"real assistant\"}]}}\n",
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"tool_result\",\"content\":\"skip tool\"}]}}\n"
        ),
    )
    .unwrap();

    let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].question, "real user");
    assert_eq!(turns[0].answer.as_deref(), Some("real assistant"));
}
