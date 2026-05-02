use super::super::*;
use crate::app::state::FocusTarget;
use crate::event::mouse;
use crate::event::normal::handle_normal_mode;
use crate::model::{
    AgentPanel, AgentState, AgentStateSource, AgentType, PreviewSource, PreviewTurn, PreviewView,
};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

const MOUSE_PREVIEW_SCROLL_DELTA: i32 = mouse::MOUSE_PREVIEW_SCROLL_DELTA;

pub(super) fn sample_panel(pane_id: &str, working_dir: &str) -> AgentPanel {
    AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: pane_id.into(),
        agent_type: AgentType::Codex,
        working_dir: working_dir.into(),
        is_active: true,
        state: AgentState::Idle,
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
    }
}

pub(super) fn test_terminal() -> ratatui::Terminal<TestBackend> {
    ratatui::Terminal::new(TestBackend::new(100, 20)).unwrap()
}

pub(super) fn left_click(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

pub(super) fn scroll_down(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

pub(super) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

mod mouse_tests;
mod preview_tab;
mod sidebar_keys;
