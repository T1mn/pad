use super::activity::merge_or_insert_thread;
use super::history_codex::build_codex_history_entry;
use crate::codex_state::CodexThreadRef;
use crate::model::{AgentState, AgentType, PreviewTurn, SessionCacheState};
use crate::session_cache::SessionCacheSnapshot;
use crate::sidebar::model::{SidebarFolder, SidebarThread};
use std::collections::HashMap;
use std::path::PathBuf;

fn folder() -> SidebarFolder {
    SidebarFolder {
        key: "/repo".into(),
        path: "/repo".into(),
        label: "repo".into(),
        updated_at: 0,
        threads: Vec::new(),
    }
}

fn codex_thread() -> CodexThreadRef {
    CodexThreadRef {
        thread_id: "sid-1".into(),
        cwd: PathBuf::from("/repo"),
        updated_at: 42,
        rollout_path: PathBuf::from("/repo/.codex/sid-1.jsonl"),
        title: Some("upstream title".into()),
        first_user_message: Some("old first prompt".into()),
        source: None,
        archived: false,
    }
}

#[test]
fn codex_history_prefers_session_cache_prompt_for_subtitle() {
    let thread = build_codex_history_entry(
        &folder(),
        &codex_thread(),
        Some(&SessionCacheSnapshot {
            agent_session_id: "sid-1".into(),
            transcript_path: Some("/repo/.codex/sid-1.jsonl".into()),
            recent_turns: vec![PreviewTurn {
                question: "newest prompt".into(),
                answer: Some("answer".into()),
            }],
            last_user_prompt: Some("newest prompt".into()),
            last_assistant_message: Some("answer".into()),
            state: SessionCacheState::Cached,
        }),
        false,
    );

    assert_eq!(thread.subtitle.as_deref(), Some("newest prompt"));
    assert_eq!(thread.last_user_prompt.as_deref(), Some("newest prompt"));
    assert_eq!(thread.cached_preview_turns.len(), 1);
}

#[test]
fn merge_or_insert_preserves_history_prompt_when_live_thread_lacks_one() {
    let mut threads = vec![SidebarThread {
        key: "live:%1".into(),
        folder_key: "/repo".into(),
        working_dir: "/repo".into(),
        folder_label: "repo".into(),
        agent_type: AgentType::Codex,
        runtime_source: None,
        session_id: Some("sid-1".into()),
        transcript_path: Some("/repo/.codex/sid-1.jsonl".into()),
        title: "live".into(),
        upstream_title: None,
        subtitle: None,
        title_override: None,
        note: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: 1,
        sort_updated_at: 1,
        live_pane_id: Some("%1".into()),
        live_location: None,
        pid: None,
        git_info: None,
        state: AgentState::Idle,
        is_active: false,
        cached_preview_turns: Vec::new(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: false,
    }];

    let history = build_codex_history_entry(
        &folder(),
        &codex_thread(),
        Some(&SessionCacheSnapshot {
            agent_session_id: "sid-1".into(),
            transcript_path: Some("/repo/.codex/sid-1.jsonl".into()),
            recent_turns: vec![PreviewTurn {
                question: "newest prompt".into(),
                answer: None,
            }],
            last_user_prompt: Some("newest prompt".into()),
            last_assistant_message: None,
            state: SessionCacheState::Cached,
        }),
        false,
    );

    merge_or_insert_thread(&mut threads, history, &[], &HashMap::new(), &HashMap::new());

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].subtitle.as_deref(), Some("newest prompt"));
    assert_eq!(
        threads[0].last_user_prompt.as_deref(),
        Some("newest prompt")
    );
    assert_eq!(threads[0].cached_preview_turns.len(), 1);
    assert_eq!(
        threads[0].session_cache_state,
        Some(SessionCacheState::Cached)
    );
}

#[test]
fn active_view_history_entries_do_not_sort_by_updated_at_without_explicit_activity() {
    let thread = build_codex_history_entry(&folder(), &codex_thread(), None, false);
    assert_eq!(thread.updated_at, 42);
    assert_eq!(thread.sort_updated_at, 0);
}

#[test]
fn archived_view_history_entries_keep_updated_at_sorting() {
    let thread = build_codex_history_entry(&folder(), &codex_thread(), None, true);
    assert_eq!(thread.updated_at, 42);
    assert_eq!(thread.sort_updated_at, 42);
}

#[test]
fn startup_sort_seed_applies_when_runtime_activity_is_missing() {
    let mut threads = Vec::new();
    let history = build_codex_history_entry(&folder(), &codex_thread(), None, false);
    let startup = HashMap::from([(String::from("codex:sid:sid-1"), 99)]);

    merge_or_insert_thread(&mut threads, history, &[], &HashMap::new(), &startup);

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].sort_updated_at, 99);
}

#[test]
fn runtime_sort_activity_overrides_startup_seed() {
    let mut threads = Vec::new();
    let history = build_codex_history_entry(&folder(), &codex_thread(), None, false);
    let runtime = HashMap::from([(String::from("codex:path:/repo/.codex/sid-1.jsonl"), 120)]);
    let startup = HashMap::from([(String::from("codex:sid:sid-1"), 99)]);

    merge_or_insert_thread(&mut threads, history, &[], &runtime, &startup);

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].sort_updated_at, 120);
}
