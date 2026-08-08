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

mod mouse_tests {
    use super::*;

    #[test]
    fn mouse_click_on_panel_row_selects_it_and_focuses_panel() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.preview.focus = FocusTarget::Preview;

        let area = Rect::new(0, 0, 100, 30);
        let regions = mouse::normal_mouse_regions(&mut app, area);
        let click = left_click(regions.panel_inner.x, regions.panel_inner.y + 1);

        mouse::handle_normal_mouse(&mut app, area, click);

        assert_eq!(app.table_state.selected(), Some(1));
        assert!(app.preview.focus == FocusTarget::Panel);
    }
    #[test]
    fn mouse_click_on_panel_row_accounts_for_scroll_offset() {
        let mut app = App::new();
        for idx in 0..6 {
            app.panels.push(sample_panel(
                &format!("%{}", idx + 1),
                &format!("/tmp/p{}", idx),
            ));
        }
        app.table_state = app.table_state.with_offset(3).with_selected(Some(3));

        let area = Rect::new(0, 0, 100, 30);
        let regions = mouse::normal_mouse_regions(&mut app, area);
        let click = left_click(regions.panel_inner.x, regions.panel_inner.y);

        mouse::handle_normal_mouse(&mut app, area, click);

        assert_eq!(app.table_state.selected(), Some(3));
    }
    #[test]
    fn mouse_click_on_second_line_of_thread_row_selects_same_item() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.sidebar.expanded_folders.insert("/tmp/alpha".into());
        app.invalidate_sidebar_visible_cache();

        let area = Rect::new(0, 0, 100, 30);
        let regions = mouse::normal_mouse_regions(&mut app, area);
        let click = left_click(regions.panel_inner.x, regions.panel_inner.y + 2);

        mouse::handle_normal_mouse(&mut app, area, click);

        assert_eq!(app.table_state.selected(), Some(1));
    }
    #[test]
    fn mouse_click_on_session_turn_selects_then_expands_on_repeat() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.preview.source = PreviewSource::Session;
        app.preview.turns = vec![
            PreviewTurn {
                question: "first".into(),
                answer: Some("one".into()),
            },
            PreviewTurn {
                question: "second".into(),
                answer: Some("two".into()),
            },
        ]
        .into();
        app.preview.view = PreviewView::SessionList;

        let area = Rect::new(0, 0, 100, 30);
        let regions = mouse::normal_mouse_regions(&mut app, area);
        let click = left_click(
            regions.preview_content_area.x,
            regions.preview_content_area.y + 4,
        );

        mouse::handle_normal_mouse(&mut app, area, click);
        assert!(app.preview.focus == FocusTarget::Preview);
        assert_eq!(app.preview.selected_turn, Some(1));
        assert_eq!(app.preview.expanded_turn, None);

        mouse::handle_normal_mouse(&mut app, area, click);
        assert_eq!(app.preview.expanded_turn, Some(1));
    }
    #[test]
    fn mouse_click_on_session_gap_does_not_change_selection() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.preview.source = PreviewSource::Session;
        app.preview.turns = vec![
            PreviewTurn {
                question: "first".into(),
                answer: Some("one".into()),
            },
            PreviewTurn {
                question: "second".into(),
                answer: Some("two".into()),
            },
        ]
        .into();
        app.preview.view = PreviewView::SessionList;
        app.preview.selected_turn = Some(0);

        let area = Rect::new(0, 0, 100, 30);
        let regions = mouse::normal_mouse_regions(&mut app, area);
        let gap_click = left_click(
            regions.preview_content_area.x,
            regions.preview_content_area.y + 3,
        );

        mouse::handle_normal_mouse(&mut app, area, gap_click);

        assert!(app.preview.focus == FocusTarget::Preview);
        assert_eq!(app.preview.selected_turn, Some(0));
        assert_eq!(app.preview.expanded_turn, None);
    }
    #[test]
    fn mouse_wheel_over_preview_scrolls_and_focuses_preview() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.preview.content = (0..20)
            .map(|idx| format!("line {}", idx))
            .collect::<Vec<_>>()
            .join("\n");

        let area = Rect::new(0, 0, 100, 20);
        let regions = mouse::normal_mouse_regions(&mut app, area);
        let wheel = scroll_down(
            regions.preview_content_area.x,
            regions.preview_content_area.y,
        );

        mouse::handle_normal_mouse(&mut app, area, wheel);

        assert!(app.preview.focus == FocusTarget::Preview);
        assert_eq!(app.preview.scroll, MOUSE_PREVIEW_SCROLL_DELTA as u16);
    }
}
mod preview_tab {
    use super::*;

    #[test]
    fn single_tab_from_detail_keeps_current_behavior_and_focuses_panel() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.preview.source = PreviewSource::Session;
        app.preview.pane_id = Some("live:%1".into());
        app.preview.turns = vec![PreviewTurn {
            question: "first".into(),
            answer: Some("one".into()),
        }]
        .into();
        app.preview.view = PreviewView::SessionDetail;
        app.preview.selected_turn = Some(0);
        app.preview.expanded_turn = Some(0);
        app.preview.focus = FocusTarget::Preview;
        app.sync_sidebar_selection();

        let mut terminal = test_terminal();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Tab)).unwrap();

        assert!(app.preview.focus == FocusTarget::Panel);
        assert_eq!(app.preview.view, PreviewView::SessionDetail);
    }
    #[test]
    fn double_tab_from_detail_restores_session_list_and_keeps_panel_focus() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.preview.source = PreviewSource::Session;
        app.preview.pane_id = Some("live:%1".into());
        app.preview.turns = vec![
            PreviewTurn {
                question: "first".into(),
                answer: Some("one".into()),
            },
            PreviewTurn {
                question: "second".into(),
                answer: Some("two".into()),
            },
        ]
        .into();
        app.preview.view = PreviewView::SessionDetail;
        app.preview.selected_turn = Some(1);
        app.preview.expanded_turn = Some(1);
        app.preview.focus = FocusTarget::Preview;
        app.sync_sidebar_selection();

        let mut terminal = test_terminal();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Tab)).unwrap();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Tab)).unwrap();

        assert!(app.preview.focus == FocusTarget::Panel);
        assert_eq!(app.preview.view, PreviewView::SessionList);
        assert_eq!(app.preview.selected_turn, Some(1));
        assert_eq!(app.preview.expanded_turn, None);
        assert_eq!(
            app.sidebar.selected_sidebar_key.as_deref(),
            Some("/tmp/alpha")
        );
    }
}
mod sidebar_keys {
    use super::*;

    #[test]
    fn space_on_selected_thread_collapses_parent_folder() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.sidebar.expanded_folders.insert("/tmp/alpha".into());
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();
        app.select_sidebar_index(1, false);

        let mut terminal = test_terminal();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();
        app.flush_pending_sidebar_space_action();

        assert!(!app.sidebar.expanded_folders.contains("/tmp/alpha"));
        assert_eq!(
            app.sidebar.selected_sidebar_key.as_deref(),
            Some("/tmp/alpha")
        );
        assert_eq!(app.table_state.selected(), Some(0));
        assert!(app.preview.focus == FocusTarget::Panel);
    }
    #[test]
    fn double_space_expands_all_folders_when_none_are_expanded() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.sync_sidebar_selection();

        let mut terminal = test_terminal();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();

        assert!(app.sidebar.expanded_folders.contains("/tmp/alpha"));
        assert!(app.sidebar.expanded_folders.contains("/tmp/beta"));
    }
    #[test]
    fn double_space_collapses_all_folders_when_any_are_expanded() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.sidebar.expanded_folders.insert("/tmp/alpha".into());
        app.sidebar.expanded_folders.insert("/tmp/beta".into());
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();
        app.select_sidebar_index(1, false);

        let mut terminal = test_terminal();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();

        assert!(app.sidebar.expanded_folders.is_empty());
        assert_eq!(
            app.sidebar.selected_sidebar_key.as_deref(),
            Some("/tmp/alpha")
        );
        assert_eq!(app.table_state.selected(), Some(0));
    }
    #[test]
    fn j_k_skip_expanded_folder_rows() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.sidebar.expanded_folders.insert("/tmp/alpha".into());
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();

        let mut terminal = test_terminal();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char('j'))).unwrap();
        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));

        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char('k'))).unwrap();
        assert_eq!(
            app.sidebar.selected_sidebar_key.as_deref(),
            Some("/tmp/beta")
        );
    }
    #[test]
    fn numeric_jump_targets_visible_threads_only() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.sidebar.expanded_folders.insert("/tmp/alpha".into());
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();

        let mut terminal = test_terminal();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char('1'))).unwrap();
        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));

        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char('2'))).unwrap();
        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));
    }

    #[cfg(unix)]
    #[test]
    fn enter_on_native_agent_thread_focuses_its_terminal_tab() {
        crate::test_support::with_temp_home("pad-sidebar", "native-agent-enter", |home| {
            let target = home.join("project");
            std::fs::create_dir_all(&target).unwrap();
            let mut app = App::new();
            app.runtime_mode = crate::runtime_mode::RuntimeMode::Native;
            app.start_native_terminal(crate::terminal_runtime::TerminalSize::new(80, 24))
                .unwrap();
            let native_pane = app
                .launch_native_agent_terminal_at(
                    "OpenCode · project",
                    "true",
                    AgentType::OpenCode,
                    target,
                    crate::terminal_runtime::TerminalSize::new(80, 24),
                )
                .unwrap();

            assert!(app.focus_terminal_tab(0));
            app.focus_panel();
            let mut terminal = test_terminal();
            handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Enter)).unwrap();

            assert_eq!(app.focused_terminal_pane_id(), Some(native_pane));
            assert!(app.terminal_is_focused());
            app.shutdown_native_terminal().unwrap();
        });
    }
}
