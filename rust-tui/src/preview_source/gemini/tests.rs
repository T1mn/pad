use super::parse_transcript;
use crate::preview_source::SessionReadMode;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_json_path(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("pad-preview-{}-{}.json", name, stamp))
}

#[test]
fn parse_gemini_transcript_skips_info_and_keeps_pairs() {
    let path = temp_json_path("gemini");
    fs::write(
        &path,
        concat!(
            "{",
            "\"sessionId\":\"sess-1\",",
            "\"kind\":\"main\",",
            "\"messages\":[",
            "{\"type\":\"info\",\"content\":\"skip me\"},",
            "{\"type\":\"user\",\"content\":[{\"text\":\"hello\"}]},",
            "{\"type\":\"gemini\",\"content\":\"world\"},",
            "{\"type\":\"user\",\"content\":{\"text\":\"second\"}},",
            "{\"type\":\"assistant\",\"content\":{\"parts\":[{\"text\":\"line 1\"},{\"text\":\"line 2\"}]}}",
            "]}"
        ),
    )
    .unwrap();

    let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].question, "second");
    assert_eq!(turns[0].answer.as_deref(), Some("line 1\nline 2"));
    assert_eq!(turns[1].question, "hello");
    assert_eq!(turns[1].answer.as_deref(), Some("world"));
}

#[test]
fn extract_session_id_from_transcript_reads_root_metadata() {
    let path = temp_json_path("gemini-meta");
    fs::write(
        &path,
        concat!(
            "{",
            "\"sessionId\":\"sess-meta-1\",",
            "\"kind\":\"main\",",
            "\"messages\":[]",
            "}"
        ),
    )
    .unwrap();

    let session_id = super::extract_session_id_from_transcript(&path);
    fs::remove_file(&path).ok();

    assert_eq!(session_id.as_deref(), Some("sess-meta-1"));
}
