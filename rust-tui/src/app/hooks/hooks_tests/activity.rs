use super::support::{stop_event, submit_event};
use crate::app::{App, APP_THREAD_ACTIVITY_MAX_ENTRIES, APP_THREAD_ACTIVITY_TTL_SECS};
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
use crate::sidebar::ThreadActivityOverride;

#[test]
fn app_thread_activity_prunes_by_ttl_and_cap() {
    let mut app = App::new();
    let now = 2_000_000i64;
    app.sidebar.app_thread_activity.insert(
        "stale".into(),
        ThreadActivityOverride {
            agent_type: AgentType::Codex,
            session_id: Some("stale".into()),
            transcript_path: None,
            working_dir: "/tmp/stale".into(),
            state: AgentState::Idle,
            is_active: false,
            last_user_prompt: None,
            last_assistant_message: None,
            updated_at: now - APP_THREAD_ACTIVITY_TTL_SECS - 1,
        },
    );
    for i in 0..(APP_THREAD_ACTIVITY_MAX_ENTRIES + 8) {
        app.sidebar.app_thread_activity.insert(
            format!("recent:{}", i),
            ThreadActivityOverride {
                agent_type: AgentType::Codex,
                session_id: Some(format!("recent:{}", i)),
                transcript_path: None,
                working_dir: "/tmp/recent".into(),
                state: AgentState::Busy,
                is_active: true,
                last_user_prompt: None,
                last_assistant_message: None,
                updated_at: now + i as i64,
            },
        );
    }

    assert!(app.prune_app_thread_activity(now));
    assert!(!app.sidebar.app_thread_activity.contains_key("stale"));
    assert_eq!(
        app.sidebar.app_thread_activity.len(),
        APP_THREAD_ACTIVITY_MAX_ENTRIES
    );
    assert!(app
        .sidebar
        .app_thread_activity
        .contains_key(&format!("recent:{}", APP_THREAD_ACTIVITY_MAX_ENTRIES + 7)));
    assert!(!app.sidebar.app_thread_activity.contains_key("recent:0"));
}

#[test]
fn pane_stop_hook_does_not_auto_reorder_sidebar() {
    let mut app = App::new();
    app.panels.push(AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        is_active: true,
        state: AgentState::Idle,
        state_source: AgentStateSource::Scanner,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: Some("session-1".into()),
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    });

    app.apply_hook_event(submit_event(Some("%1")));
    assert!(app.sidebar.thread_sort_activity.is_empty());

    app.apply_hook_event(stop_event("%1"));
    assert!(app.sidebar.thread_sort_activity.is_empty());
}

#[test]
fn app_stop_hook_does_not_auto_reorder_sidebar() {
    let mut app = App::new();

    app.apply_hook_event(submit_event(None));
    assert!(app.sidebar.thread_sort_activity.is_empty());

    let mut event = stop_event("%1");
    event.tmux.pane_id = None;
    event.cwd = Some("/tmp/demo".into());
    app.apply_hook_event(event);
    assert!(app.sidebar.thread_sort_activity.is_empty());
}
