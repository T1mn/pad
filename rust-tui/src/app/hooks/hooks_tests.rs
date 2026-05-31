use crate::app::state::FocusTarget;
use crate::app::{App, APP_THREAD_ACTIVITY_MAX_ENTRIES, APP_THREAD_ACTIVITY_TTL_SECS};
use crate::hook::{HookEvent, HookTmuxInfo};
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
use crate::notify::NotificationRequest;
use crate::sidebar::ThreadActivityOverride;
use rusqlite::Connection;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_stamp() -> u128 {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        + NEXT_ID.fetch_add(1, Ordering::Relaxed) as u128
}

fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("home env lock");
    let home = std::env::temp_dir().join(format!("pad-hooks-{name}-{}", temp_stamp()));
    std::fs::create_dir_all(&home).expect("create temp home");
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    let result = f(&home);
    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    std::fs::remove_dir_all(&home).ok();
    result
}

fn create_codex_threads_db(path: &Path) {
    let connection = Connection::open(path).expect("open sqlite");
    connection
        .execute_batch(
            "CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    cwd TEXT NOT NULL,
                    updated_at INTEGER NOT NULL,
                    rollout_path TEXT NOT NULL,
                    title TEXT,
                    first_user_message TEXT,
                    source TEXT,
                    archived INTEGER NOT NULL DEFAULT 0,
                    archived_at INTEGER
                );",
        )
        .expect("create threads table");
}

fn insert_codex_thread(path: &Path, thread_id: &str, title: &str) {
    let connection = Connection::open(path).expect("open sqlite");
    connection
            .execute(
                "INSERT INTO threads (
                    id, cwd, updated_at, rollout_path, title, first_user_message, source, archived, archived_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, NULL)",
                (
                    thread_id,
                    "/tmp/demo",
                    1_i64,
                    format!("/tmp/{thread_id}.jsonl"),
                    title,
                    "old first prompt",
                    "cli",
                ),
            )
            .expect("insert thread");
}

fn stop_event(pane_id: &str) -> HookEvent {
    HookEvent {
        event: "stop".into(),
        turn_id: Some("turn-1".into()),
        session_id: Some("session-1".into()),
        transcript_path: None,
        cwd: None,
        prompt: None,
        last_assistant_message: Some("done".into()),
        timestamp: None,
        tmux: HookTmuxInfo {
            pane_id: Some(pane_id.into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: Some("/tmp/demo".into()),
        },
    }
}

fn submit_event(pane_id: Option<&str>) -> HookEvent {
    HookEvent {
        event: "user_prompt_submit".into(),
        turn_id: Some("turn-1".into()),
        session_id: Some("session-1".into()),
        transcript_path: None,
        cwd: Some("/tmp/demo".into()),
        prompt: Some("ship it".into()),
        last_assistant_message: None,
        timestamp: None,
        tmux: HookTmuxInfo {
            pane_id: pane_id.map(str::to_string),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: Some("/tmp/demo".into()),
        },
    }
}

#[test]
fn stop_hook_marks_panel_unread_when_panel_item_is_not_focused() {
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
        state: AgentState::Busy,
        state_source: AgentStateSource::Scanner,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    });
    app.table_state.select(Some(0));
    app.preview.focus = FocusTarget::Preview;

    app.apply_hook_event(stop_event("%1"));

    assert!(app.panels[0].has_unread_stop);
}

#[test]
fn focusing_panel_clears_unread_stop_marker() {
    let mut app = App::new();
    app.panels.push(AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        is_active: false,
        state: AgentState::Waiting,
        state_source: AgentStateSource::Hook,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: Some("done".into()),
        has_unread_stop: true,
    });
    app.table_state.select(Some(0));
    app.preview.focus = FocusTarget::Preview;

    app.focus_panel();

    assert!(!app.panels[0].has_unread_stop);
}

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

#[test]
fn completion_notification_uses_prompt_when_lookup_is_unavailable() {
    let request = super::build_completion_notification(
        &AgentType::Codex,
        Some("missing-session"),
        Some("Ship the relay settings redesign with a compact layout"),
        Some("/tmp/demo"),
    );

    assert_eq!(request.title, "PAD · Codex complete");
    assert_eq!(
        request.body,
        "Ship the relay settings redesign with a compact layout"
    );
}

#[test]
fn completion_notification_falls_back_to_workdir_name() {
    let request = super::build_completion_notification(
        &AgentType::OpenCode,
        None,
        None,
        Some("/tmp/pad-demo"),
    );

    assert_eq!(
        request,
        NotificationRequest {
            title: "PAD · OpenCode complete".into(),
            body: "pad-demo".into(),
        }
    );
}

#[test]
fn completion_notification_truncates_long_text() {
    let body = super::completion_notification_body(
            &AgentType::Unknown,
            None,
            Some(
                "this is a very long prompt that should be truncated before it reaches the desktop notification surface because otherwise it becomes noisy",
            ),
            None,
        );

    assert!(body.ends_with("..."));
    assert!(body.chars().count() <= 75);
}

#[test]
fn completion_notification_prefers_latest_prompt_over_persisted_codex_title() {
    with_temp_home("notify-latest-prompt", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let db_path = codex_dir.join("state_5.sqlite");
        create_codex_threads_db(&db_path);
        insert_codex_thread(&db_path, "session-1", "Very old title");

        let request = super::build_completion_notification(
            &AgentType::Codex,
            Some("session-1"),
            Some("Latest prompt should win over the old title"),
            Some("/tmp/demo"),
        );

        assert_eq!(request.body, "Latest prompt should win over the old title");
    });
}

#[test]
fn stop_hook_emits_completion_sound_event() {
    with_temp_home("completion-sound", |_home| {
        crate::sound::with_test_sound_capture(|| {
            let _ = crate::sound::take_test_playbacks();
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
                state: AgentState::Busy,
                state_source: AgentStateSource::Scanner,
                transcript_path: None,
                cached_preview_turns: Default::default(),
                session_cache_state: None,
                git_info: None,
                pid: None,
                start_time: None,
                agent_session_id: Some("session-1".into()),
                last_user_prompt: Some("ship it".into()),
                last_assistant_message: None,
                has_unread_stop: false,
            });

            app.apply_hook_event(stop_event("%1"));

            assert_eq!(
                crate::sound::take_test_playbacks(),
                vec![crate::sound::TestPlayback {
                    event: Some(crate::sound::SoundEvent::Completion),
                    preset: "glass".into(),
                }]
            );
        });
    });
}

#[test]
fn stop_hook_adds_completion_to_notification_inbox() {
    let mut app = App::new();
    app.notification_inbox.entries.clear();
    app.panels.push(AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        is_active: true,
        state: AgentState::Busy,
        state_source: AgentStateSource::Scanner,
        transcript_path: Some("/tmp/demo/transcript.jsonl".into()),
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: Some("session-1".into()),
        last_user_prompt: Some("ship it".into()),
        last_assistant_message: None,
        has_unread_stop: false,
    });

    app.apply_hook_event(stop_event("%1"));

    assert_eq!(app.notification_inbox.entries.len(), 1);
    let entry = &app.notification_inbox.entries[0];
    assert_eq!(entry.agent_type, "codex");
    assert_eq!(entry.event, "stop");
    assert_eq!(entry.body, "ship it");
    assert_eq!(entry.pane_id.as_deref(), Some("%1"));
    assert!(!entry.read);
}
