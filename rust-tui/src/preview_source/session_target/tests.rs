use super::target::SessionTarget;
use super::{persistence_panel_from_request, resolved_session_id_for_request};
use crate::model::{AgentState, AgentType, PreviewSessionOrigin, PreviewTurn, SessionCacheState};
use crate::preview_source::PreviewRequest;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_json_path(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("pad-gemini-target-{}-{}.json", name, stamp))
}

fn base_request() -> PreviewRequest {
    PreviewRequest {
        target_key: "gemini:test".into(),
        live_pane_id: Some("%7".into()),
        agent_type: AgentType::Gemini,
        working_dir: "/tmp/gemini".into(),
        state: AgentState::Idle,
        transcript_path: None,
        cached_preview_turns: vec![PreviewTurn {
            question: "hello".into(),
            answer: Some("world".into()),
        }]
        .into(),
        session_cache_state: Some(SessionCacheState::Cached),
        agent_session_id: None,
        session_origin: Some(PreviewSessionOrigin::Pane),
        persist_resolved_session: true,
        known_updated_at: None,
    }
}

#[test]
fn gemini_session_id_can_be_read_from_transcript_path() {
    let path = temp_json_path("session-id");
    fs::write(
        &path,
        concat!(
            "{",
            "\"sessionId\":\"gemini-session-1\",",
            "\"kind\":\"main\",",
            "\"messages\":[]",
            "}"
        ),
    )
    .unwrap();

    let mut request = base_request();
    request.transcript_path = Some(path.to_string_lossy().to_string());

    let session_id = resolved_session_id_for_request(&request, None);
    fs::remove_file(&path).ok();

    assert_eq!(session_id.as_deref(), Some("gemini-session-1"));
}

#[test]
fn persistence_panel_uses_resolved_target_session_id() {
    let request = base_request();
    let target = SessionTarget {
        origin: PreviewSessionOrigin::Pane,
        session_id: Some("gemini-session-2".into()),
        transcript_path: PathBuf::from("/tmp/gemini-session-2.json"),
        updated_at: Some(42),
    };

    let panel = persistence_panel_from_request(&request, &target).unwrap();
    assert_eq!(panel.agent_session_id.as_deref(), Some("gemini-session-2"));
    assert_eq!(
        panel.transcript_path.as_deref(),
        Some("/tmp/gemini-session-2.json")
    );
}
