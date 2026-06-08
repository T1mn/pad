use super::super::bindings::find_snapshot_for_panel;
use super::super::model::{
    snapshot_from_record, CachedPaneBinding, CachedSessionRecord, SessionCacheIndex,
};
use super::super::util::now_ts;
use super::support::panel;
use crate::model::{PreviewTurn, SessionCacheState};

fn record(id: &str, transcript_path: &str, question: &str) -> CachedSessionRecord {
    CachedSessionRecord {
        agent_session_id: id.to_string(),
        agent_type: "codex".to_string(),
        transcript_path: Some(transcript_path.to_string()),
        recent_turns: vec![PreviewTurn {
            question: question.to_string(),
            answer: None,
        }],
        last_user_prompt: None,
        last_assistant_message: None,
        last_seen_at: 1,
        updated_at: 1,
        last_source: "hook".to_string(),
    }
}

fn binding(session_id: &str, pane_id: &str) -> CachedPaneBinding {
    CachedPaneBinding {
        agent_session_id: session_id.to_string(),
        pane_id: pane_id.to_string(),
        pane_pid: Some(format!("pid-{pane_id}")),
        session_name: "dev".to_string(),
        window_index: "1".to_string(),
        pane_index: "0".to_string(),
        path: "/repo".to_string(),
        agent_type: "codex".to_string(),
        updated_at: 1,
    }
}

#[test]
fn fallback_match_is_ambiguous_when_multiple_sessions_share_same_slot() {
    let now = now_ts();
    let index = SessionCacheIndex {
        version: 1,
        sessions: vec![
            record("s1", "/tmp/a.jsonl", "q1"),
            record("s2", "/tmp/b.jsonl", "q2"),
        ],
        pane_bindings: vec![
            CachedPaneBinding {
                updated_at: now,
                ..binding("s1", "%1")
            },
            CachedPaneBinding {
                updated_at: now,
                ..binding("s2", "%2")
            },
        ],
    };

    assert!(find_snapshot_for_panel(&index, &panel("%9", "dev", "1", "0", "/repo")).is_none());
}

#[test]
fn exact_pane_match_wins_even_if_slot_history_is_ambiguous() {
    let record = record("s1", "/tmp/a.jsonl", "q1");
    let index = SessionCacheIndex {
        version: 1,
        sessions: vec![record.clone()],
        pane_bindings: vec![binding("s1", "%1")],
    };

    let snapshot =
        find_snapshot_for_panel(&index, &panel("%1", "other", "9", "9", "/else")).unwrap();
    assert_eq!(
        snapshot,
        snapshot_from_record(&record, SessionCacheState::Cached)
    );
}

#[test]
fn fallback_match_allows_duplicate_bindings_for_same_session_id() {
    let now = now_ts();
    let record = record("s1", "/tmp/a.jsonl", "q1");
    let index = SessionCacheIndex {
        version: 1,
        sessions: vec![record.clone()],
        pane_bindings: vec![
            CachedPaneBinding {
                updated_at: now,
                ..binding("s1", "%1")
            },
            CachedPaneBinding {
                updated_at: now,
                ..binding("s1", "%2")
            },
        ],
    };

    let snapshot = find_snapshot_for_panel(&index, &panel("%9", "dev", "1", "0", "/repo"));
    assert_eq!(
        snapshot,
        Some(snapshot_from_record(&record, SessionCacheState::Cached))
    );
}

#[test]
fn exact_match_requires_recent_binding_when_pid_is_missing() {
    let index = SessionCacheIndex {
        version: 1,
        sessions: vec![record("s1", "/tmp/a.jsonl", "q1")],
        pane_bindings: vec![CachedPaneBinding {
            pane_pid: None,
            ..binding("s1", "%1")
        }],
    };

    assert!(find_snapshot_for_panel(&index, &panel("%1", "dev", "1", "0", "/repo")).is_none());
}

#[test]
fn exact_match_keeps_working_for_stale_binding_when_pane_pid_matches() {
    let record = record("s1", "/tmp/a.jsonl", "q1");
    let index = SessionCacheIndex {
        version: 1,
        sessions: vec![record.clone()],
        pane_bindings: vec![CachedPaneBinding {
            session_name: "old".to_string(),
            window_index: "9".to_string(),
            pane_index: "9".to_string(),
            path: "/else".to_string(),
            ..binding("s1", "%1")
        }],
    };

    let snapshot = find_snapshot_for_panel(&index, &panel("%1", "dev", "1", "0", "/repo")).unwrap();
    assert_eq!(
        snapshot,
        snapshot_from_record(&record, SessionCacheState::Cached)
    );
}
