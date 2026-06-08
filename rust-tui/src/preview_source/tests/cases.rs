use super::super::{
    load_preview, preview_refresh_interval_ms_for_request, session_loader::load_session_preview,
    PreviewRequest,
};
use crate::i18n::Locale;
use crate::model::{
    AgentState, AgentType, PreviewSessionOrigin, PreviewSource, PreviewTurn, SessionCacheState,
};

#[test]
fn request_refresh_interval_is_adaptive_to_state_and_origin() {
    let base = PreviewRequest {
        target_key: "demo".into(),
        live_pane_id: Some("%1".into()),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        state: AgentState::Idle,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        agent_session_id: None,
        session_origin: None,
        persist_resolved_session: false,
        known_updated_at: None,
    };

    assert_eq!(preview_refresh_interval_ms_for_request(&base), 2500);

    let mut waiting = base.clone();
    waiting.state = AgentState::Waiting;
    assert_eq!(preview_refresh_interval_ms_for_request(&waiting), 1200);

    let mut busy = base.clone();
    busy.state = AgentState::Busy;
    assert_eq!(preview_refresh_interval_ms_for_request(&busy), 1000);

    let mut app_idle = base.clone();
    app_idle.live_pane_id = None;
    app_idle.session_origin = Some(PreviewSessionOrigin::App);
    assert_eq!(preview_refresh_interval_ms_for_request(&app_idle), 1200);

    let mut history_idle = base;
    history_idle.live_pane_id = None;
    history_idle.session_origin = None;
    assert_eq!(preview_refresh_interval_ms_for_request(&history_idle), 4000);
}

#[test]
fn confirmed_cached_preview_returns_without_resolving_target() {
    let request = PreviewRequest {
        target_key: "codex:test".into(),
        live_pane_id: None,
        agent_type: AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        state: AgentState::Idle,
        transcript_path: Some("/definitely/missing/rollout.jsonl".into()),
        cached_preview_turns: vec![PreviewTurn {
            question: "hello".into(),
            answer: Some("world".into()),
        }]
        .into(),
        session_cache_state: Some(SessionCacheState::Confirmed),
        agent_session_id: Some("019d2e9d-9010-79a2-b381-52af55b198f6".into()),
        session_origin: Some(PreviewSessionOrigin::App),
        persist_resolved_session: false,
        known_updated_at: Some(42),
    };

    let preview = load_session_preview(&request, Locale::ZhCN).unwrap();
    assert_eq!(preview.turns.len(), 1);
    assert_eq!(preview.turns[0].question, "hello");
    assert_eq!(preview.turns[0].answer.as_deref(), Some("world"));
    assert_eq!(preview.updated_at, Some(42));
}

#[test]
fn session_preview_update_keeps_content_empty_for_memory() {
    let request = PreviewRequest {
        target_key: "codex:test".into(),
        live_pane_id: None,
        agent_type: AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        state: AgentState::Idle,
        transcript_path: None,
        cached_preview_turns: vec![PreviewTurn {
            question: "hello".into(),
            answer: Some("world".into()),
        }]
        .into(),
        session_cache_state: Some(SessionCacheState::Confirmed),
        agent_session_id: Some("session-1".into()),
        session_origin: Some(PreviewSessionOrigin::App),
        persist_resolved_session: false,
        known_updated_at: Some(42),
    };

    let update = load_preview(&request, "session", Locale::ZhCN);

    assert_eq!(update.source, PreviewSource::Session);
    assert_eq!(update.turns.len(), 1);
    assert!(update.content.is_empty());
}
