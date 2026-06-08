use super::{leaf_name, panel_display_title};
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_home(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("pad-chat-backend-{name}-{stamp}"))
}

fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock backend tests");
    let home = temp_home(name);
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let result = f(&home);

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);

    result
}

fn sample_panel(session_id: Option<&str>) -> AgentPanel {
    AgentPanel {
        session: "0".into(),
        window: "zsh".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%42".into(),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/rust-tui".into(),
        is_active: false,
        state: AgentState::Idle,
        state_source: AgentStateSource::Scanner,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: session_id.map(str::to_string),
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    }
}

#[test]
fn panel_display_title_uses_thread_meta_title_override() {
    with_temp_home("title-override", |_| {
        crate::thread_meta::upsert_thread_meta(
            "codex",
            "session-1",
            Some("  Renamed title  \nignored line"),
            None,
            false,
        )
        .expect("write thread meta");

        let panel = sample_panel(Some("session-1"));
        assert_eq!(panel_display_title(&panel), "Renamed title");
    });
}

#[test]
fn panel_display_title_falls_back_to_working_dir_leaf() {
    let panel = sample_panel(None);
    assert_eq!(panel_display_title(&panel), leaf_name(&panel.working_dir));
}
