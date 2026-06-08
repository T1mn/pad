use super::super::model::SessionCacheSnapshot;
use super::super::preload::{
    apply_snapshot_to_panel, latest_turn_missing_answer, panel_needs_preload,
};
use super::support::panel;
use crate::model::{AgentState, AgentStateSource, AgentType, PreviewTurn, SessionCacheState};

#[test]
fn latest_unanswered_turn_restores_busy_state() {
    let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
    let snapshot = SessionCacheSnapshot {
        agent_session_id: "s1".to_string(),
        transcript_path: Some("/tmp/a.jsonl".to_string()),
        recent_turns: vec![PreviewTurn {
            question: "still running".to_string(),
            answer: None,
        }]
        .into(),
        last_user_prompt: Some("still running".to_string()),
        last_assistant_message: None,
        state: SessionCacheState::Cached,
    };

    apply_snapshot_to_panel(&mut restored_panel, &snapshot);

    assert_eq!(restored_panel.state, AgentState::Busy);
    assert_eq!(restored_panel.state_source, AgentStateSource::Hook);
    assert!(restored_panel.is_active);
}

#[test]
fn answered_latest_turn_does_not_force_busy_state() {
    let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
    let snapshot = SessionCacheSnapshot {
        agent_session_id: "s1".to_string(),
        transcript_path: Some("/tmp/a.jsonl".to_string()),
        recent_turns: vec![PreviewTurn {
            question: "done".to_string(),
            answer: Some("finished".to_string()),
        }]
        .into(),
        last_user_prompt: Some("done".to_string()),
        last_assistant_message: Some("finished".to_string()),
        state: SessionCacheState::Cached,
    };

    apply_snapshot_to_panel(&mut restored_panel, &snapshot);

    assert_eq!(restored_panel.state, AgentState::Idle);
    assert_eq!(restored_panel.state_source, AgentStateSource::Scanner);
    assert!(!restored_panel.is_active);
}

#[test]
fn preload_index_is_needed_only_for_supported_empty_panels() {
    let mut empty = panel("%1", "dev", "1", "0", "/repo");
    assert!(panel_needs_preload(&empty));

    empty.agent_session_id = Some("session-1".to_string());
    assert!(!panel_needs_preload(&empty));

    let mut unsupported = panel("%2", "dev", "1", "1", "/repo");
    unsupported.agent_type = AgentType::Aider;
    assert!(!panel_needs_preload(&unsupported));
}

#[test]
fn latest_turn_missing_answer_only_when_newest_turn_is_unresolved() {
    assert!(latest_turn_missing_answer(&[PreviewTurn {
        question: "pending".to_string(),
        answer: None,
    }]));
    assert!(!latest_turn_missing_answer(&[PreviewTurn {
        question: "done".to_string(),
        answer: Some("answer".to_string()),
    }]));
    assert!(!latest_turn_missing_answer(&[
        PreviewTurn {
            question: "done".to_string(),
            answer: Some("answer".to_string()),
        },
        PreviewTurn {
            question: "old pending".to_string(),
            answer: None,
        },
    ]));
}

#[test]
fn apply_snapshot_to_panel_normalizes_old_codex_image_placeholders() {
    let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
    let snapshot = SessionCacheSnapshot {
        agent_session_id: "s1".to_string(),
        transcript_path: Some("/tmp/a.jsonl".to_string()),
        recent_turns: vec![PreviewTurn {
            question: "<image name=[Image #1]>\n</image>\n[Image #1] 为什么有黑边？".to_string(),
            answer: Some("因为边框".to_string()),
        }]
        .into(),
        last_user_prompt: Some(
            "<image name=[Image #1]>\n</image>\n[Image #1] 为什么有黑边？".to_string(),
        ),
        last_assistant_message: Some("因为边框".to_string()),
        state: SessionCacheState::Cached,
    };

    apply_snapshot_to_panel(&mut restored_panel, &snapshot);

    assert_eq!(
        restored_panel.cached_preview_turns[0].question,
        "[Image x1] 为什么有黑边？"
    );
    assert_eq!(
        restored_panel.last_user_prompt.as_deref(),
        Some("[Image x1] 为什么有黑边？")
    );
}
