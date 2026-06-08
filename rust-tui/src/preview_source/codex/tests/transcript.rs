use super::super::parse_transcript;
use super::support::temp_jsonl_path;
use crate::preview_source::SessionReadMode;
use std::fs;

#[test]
fn parse_codex_transcript_extracts_recent_messages() {
    let path = temp_jsonl_path("codex");
    fs::write(
        &path,
        concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"developer\",\"content\":[{\"type\":\"input_text\",\"text\":\"skip\"}]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hello\"}]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"world\"}]}}\n"
        ),
    )
    .unwrap();

    let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].question, "hello");
    assert_eq!(turns[0].answer.as_deref(), Some("world"));
}

#[test]
fn parse_codex_transcript_backfills_beyond_six_turns() {
    let path = temp_jsonl_path("codex-history");
    let mut content = String::new();
    for idx in 0..8 {
        content.push_str(&format!(
            "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"q{idx}\"}}]}}}}\n"
        ));
        content.push_str(&format!(
            "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{{\"type\":\"output_text\",\"text\":\"a{idx}\"}}]}}}}\n"
        ));
    }
    fs::write(&path, content).unwrap();

    let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(turns.len(), 8);
    assert_eq!(turns[0].question, "q7");
    assert_eq!(turns[7].question, "q0");
}

#[test]
fn parse_codex_transcript_keeps_latest_real_user_turns() {
    let path = temp_jsonl_path("codex-history-limit");
    let mut content = String::new();
    for idx in 0..60 {
        content.push_str(&format!(
            "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"q{idx}\"}}]}}}}\n"
        ));
        content.push_str(&format!(
            "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{{\"type\":\"output_text\",\"text\":\"a{idx}\"}}]}}}}\n"
        ));
    }
    for _ in 0..20 {
        content.push_str(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<environment_context>\\n  <cwd>/tmp/demo</cwd>\\n</environment_context>\"}]}}\n",
        );
    }
    fs::write(&path, content).unwrap();

    let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(
        turns.len(),
        crate::session_cache::SESSION_HISTORY_TURN_LIMIT
    );
    assert_eq!(turns[0].question, "q59");
    assert_eq!(turns[49].question, "q10");
}

#[test]
fn parse_codex_transcript_includes_subagent_events_in_main_turn() {
    let path = temp_jsonl_path("codex-subagent");
    fs::write(
        &path,
        concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"run 2 subagents\"}]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"spawn_agent\",\"arguments\":\"{\\\"agent_type\\\":\\\"explorer\\\",\\\"task_name\\\":\\\"audit_event_rs\\\"}\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<subagent_notification>\\n{\\\"agent_path\\\":\\\"/root/audit_event_rs\\\",\\\"status\\\":{\\\"completed\\\":\\\"`src/event.rs` is structurally overloaded and should be split into layers.\\\"}}\\n</subagent_notification>\"}]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"merged result\"}]}}\n"
        ),
    )
    .unwrap();

    let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(turns.len(), 1);
    let answer = turns[0].answer.as_deref().unwrap_or("");
    assert_eq!(turns[0].question, "run 2 subagents");
    assert!(answer.contains("[subagent/start][explorer] audit_event_rs"));
    assert!(answer.contains("[subagent/completed] audit_event_rs"));
    assert!(answer.contains("merged result"));
    assert!(!answer.contains("<subagent_notification>"));
}

#[test]
fn parse_codex_transcript_normalizes_single_image_user_message() {
    let path = temp_jsonl_path("codex-image-single");
    fs::write(
        &path,
        concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[",
            "{\"type\":\"input_text\",\"text\":\"<image name=[Image #1]>\"},",
            "{\"type\":\"input_image\",\"image_url\":\"file:///tmp/1.png\"},",
            "{\"type\":\"input_text\",\"text\":\"</image>\"},",
            "{\"type\":\"input_text\",\"text\":\"[Image #1] 为什么 settings 底部有黑边？\"}",
            "]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"看一下\"}]}}\n"
        ),
    )
    .unwrap();

    let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(turns[0].question, "[Image x1] 为什么 settings 底部有黑边？");
}

#[test]
fn parse_codex_transcript_normalizes_multiple_image_user_message() {
    let path = temp_jsonl_path("codex-image-multi");
    fs::write(
        &path,
        concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[",
            "{\"type\":\"input_text\",\"text\":\"<image name=[Image #1]>\"},",
            "{\"type\":\"input_image\",\"image_url\":\"file:///tmp/1.png\"},",
            "{\"type\":\"input_text\",\"text\":\"</image>\"},",
            "{\"type\":\"input_text\",\"text\":\"<image name=[Image #2]>\"},",
            "{\"type\":\"input_image\",\"image_url\":\"file:///tmp/2.png\"},",
            "{\"type\":\"input_text\",\"text\":\"</image>\"},",
            "{\"type\":\"input_text\",\"text\":\"[Image #1] 左侧不对，[Image #2] 右侧也不对\"}",
            "]}}\n"
        ),
    )
    .unwrap();

    let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(turns[0].question, "[Image x2] 左侧不对， 右侧也不对");
}

#[test]
fn parse_codex_transcript_skips_context_only_user_messages() {
    let path = temp_jsonl_path("codex-context-filter");
    fs::write(
        &path,
        concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<environment_context>\\n  <cwd>/tmp/demo</cwd>\\n</environment_context>\"}]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"real question\"}]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<turn_aborted>\\ninterrupted\\n</turn_aborted>\"}]}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"real answer\"}]}}\n"
        ),
    )
    .unwrap();

    let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].question, "real question");
    assert_eq!(turns[0].answer.as_deref(), Some("real answer"));
}
