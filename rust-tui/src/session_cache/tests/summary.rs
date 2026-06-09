use super::super::model::{CachedPaneBinding, CachedSessionRecord, SessionCacheIndex};
use crate::model::PreviewTurn;

fn record(id: &str, updated_at: i64) -> CachedSessionRecord {
    CachedSessionRecord {
        agent_session_id: id.to_string(),
        agent_type: "codex".to_string(),
        transcript_path: Some(format!("/tmp/{id}.jsonl")),
        recent_turns: vec![PreviewTurn {
            question: format!("question {id}"),
            answer: None,
        }],
        last_user_prompt: Some(format!("prompt {id}")),
        last_assistant_message: Some(format!("answer {id}")),
        last_seen_at: updated_at,
        updated_at,
        last_source: "hook".to_string(),
    }
}

fn binding(session_id: &str, pane_id: &str, path: &str, updated_at: i64) -> CachedPaneBinding {
    CachedPaneBinding {
        agent_session_id: session_id.to_string(),
        pane_id: pane_id.to_string(),
        pane_pid: Some(format!("pid-{pane_id}")),
        session_name: "dev".to_string(),
        window_index: "1".to_string(),
        pane_index: "0".to_string(),
        path: path.to_string(),
        agent_type: "codex".to_string(),
        updated_at,
    }
}

fn with_index(index: &SessionCacheIndex, f: impl FnOnce()) {
    crate::test_support::with_temp_home("pad-session-summary", "index", |_| {
        let path = crate::paths::sessions_index_path();
        std::fs::create_dir_all(path.parent().expect("index parent")).expect("create sessions dir");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(index).expect("serialize index"),
        )
        .expect("write index");
        f();
    });
}

#[test]
fn find_cached_session_returns_matching_summary_with_latest_binding() {
    let index = SessionCacheIndex {
        version: 1,
        sessions: vec![record("old", 1), record("target", 2)],
        pane_bindings: vec![
            binding("target", "%1", "/older", 10),
            binding("target", "%2", "/newer", 20),
            binding("old", "%3", "/old", 30),
        ],
    };

    with_index(&index, || {
        let summary = super::super::find_cached_session(" target ").expect("summary");

        assert_eq!(summary.agent_session_id, "target");
        assert_eq!(summary.working_dir.as_deref(), Some("/newer"));
        assert_eq!(summary.pane_id.as_deref(), Some("%2"));
        assert_eq!(summary.last_user_prompt.as_deref(), Some("prompt target"));
    });
}

#[test]
fn list_cached_sessions_and_find_use_same_summary_shape() {
    let index = SessionCacheIndex {
        version: 1,
        sessions: vec![record("target", 2)],
        pane_bindings: vec![binding("target", "%1", "/repo", 10)],
    };

    with_index(&index, || {
        let listed = super::super::list_cached_sessions()
            .into_iter()
            .find(|session| session.agent_session_id == "target")
            .expect("listed summary");
        let found = super::super::find_cached_session("target").expect("found summary");

        assert_eq!(found, listed);
    });
}
