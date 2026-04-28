use crate::app::state::Mode;
use crate::app::App;
use crate::log_debug;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;
use std::time::Duration;

#[cfg(test)]
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
#[cfg(test)]
use ratatui::layout::Rect;

#[cfg_attr(test, allow(dead_code))]
mod attach;
mod event_pipeline;
mod key_pipeline;
mod loop_core;
mod loop_state;
mod modes;
mod mouse;
mod mouse_pipeline;
mod refresh_pipeline;

#[cfg(test)]
const MOUSE_PREVIEW_SCROLL_DELTA: i32 = mouse::MOUSE_PREVIEW_SCROLL_DELTA;
pub fn restore_tmux_bindings(app: &mut App) {
    attach::restore_tmux_bindings(app);
}

fn pad_focus_state() -> Option<(String, String)> {
    let pad_pane_id = std::env::var("TMUX_PANE").ok()?;
    let current_pane_id = attach::current_tmux_pane_id()?;
    Some((pad_pane_id, current_pane_id))
}

pub async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop_core::run_app(terminal, app).await
}

#[cfg(not(test))]
fn handle_normal_mode(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: KeyEvent,
) -> io::Result<()> {
    handle_normal_mode_impl(terminal, app, key, handle_attach)
}

#[cfg(test)]
fn handle_normal_mode<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    key: KeyEvent,
) -> io::Result<()> {
    handle_normal_mode_impl(terminal, app, key, |_terminal, _app| {
        Err(io::Error::other("attach is not supported in event tests"))
    })
}

fn handle_normal_mode_impl<B: Backend, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    key: KeyEvent,
    mut attach_fn: F,
) -> io::Result<()>
where
    F: FnMut(&mut Terminal<B>, &mut App) -> io::Result<()>,
{
    const DOUBLE_SPACE_WINDOW: Duration = Duration::from_millis(250);

    log_debug!(
        "normal_mode key={:?} show_tree={} panels={}",
        key.code,
        app.sidebar.show_tree,
        app.panels.len()
    );

    let is_tab = matches!(key.code, KeyCode::Tab);
    let is_space = matches!(key.code, KeyCode::Char(' '));

    if !is_space {
        app.flush_pending_sidebar_space_action();
    }

    if !app.sidebar.show_tree && matches!(key.code, KeyCode::Tab) {
        handle_preview_tab(app);
        return Ok(());
    }

    if !is_tab {
        app.clear_panel_tab();
        app.clear_detail_exit_tab();
    }

    if key.code == KeyCode::F(2) && app.open_thread_title_editor() {
        return Ok(());
    }

    if key.code == KeyCode::Char('T') && app.open_thread_tags_editor() {
        return Ok(());
    }

    if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.open_tree_in_home();
        return Ok(());
    }

    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.mode = Mode::Search;
        app.is_searching = true;
        app.search_query.clear();
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();
        app.dirty = true;
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
            return Ok(());
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.refresh_panels();
            app.dirty = true;
            return Ok(());
        }
        KeyCode::Char('/') => {
            app.open_settings_search();
            return Ok(());
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
            app.dirty = true;
            return Ok(());
        }
        KeyCode::F(1) => {
            app.toggle_settings();
            app.dirty = true;
            return Ok(());
        }
        KeyCode::Char('t') => {
            app.toggle_tree();
            return Ok(());
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.toggle_display_session_scope_view();
            return Ok(());
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.open_fuzzy_picker();
            return Ok(());
        }
        KeyCode::Char('Z') => {
            app.toggle_archived_threads_view();
            return Ok(());
        }
        KeyCode::Char('A') => {
            let _ = app.request_archive_selected_thread();
            return Ok(());
        }
        KeyCode::Char('U') => {
            let _ = app.request_unarchive_selected_thread();
            return Ok(());
        }
        KeyCode::Char('1') => {
            app.jump_to(0);
            return Ok(());
        }
        KeyCode::Char('2') => {
            app.jump_to(1);
            return Ok(());
        }
        KeyCode::Char('3') => {
            app.jump_to(2);
            return Ok(());
        }
        KeyCode::Char('4') => {
            app.jump_to(3);
            return Ok(());
        }
        KeyCode::Char('5') => {
            app.jump_to(4);
            return Ok(());
        }
        KeyCode::Char('6') => {
            app.jump_to(5);
            return Ok(());
        }
        KeyCode::Char('7') => {
            app.jump_to(6);
            return Ok(());
        }
        KeyCode::Char('8') => {
            app.jump_to(7);
            return Ok(());
        }
        KeyCode::Char('9') => {
            app.jump_to(8);
            return Ok(());
        }
        _ => {}
    }

    if app.preview_is_focused() {
        match key.code {
            KeyCode::Esc => {
                app.step_back_preview_focus();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.scroll_preview_by(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.scroll_preview_by(-1);
            }
            KeyCode::Char('J') => {
                if app.has_session_preview_turns() {
                    app.select_next_preview_turn();
                } else {
                    app.scroll_preview_by(10);
                }
            }
            KeyCode::Char('K') => {
                if app.has_session_preview_turns() {
                    app.select_previous_preview_turn();
                } else {
                    app.scroll_preview_by(-10);
                }
            }
            KeyCode::PageDown => {
                app.scroll_preview_by(20);
            }
            KeyCode::PageUp => {
                app.scroll_preview_by(-20);
            }
            KeyCode::Home => {
                app.scroll_preview_to_top();
            }
            KeyCode::End => {
                app.scroll_preview_to_bottom();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let _ = app.toggle_preview_turn_expanded();
            }
            _ => {}
        }
        return Ok(());
    }

    if is_space && !app.sidebar.show_tree {
        if app.pending_sidebar_space_action_is_active() {
            app.clear_pending_sidebar_space_action();
            let _ = app.toggle_all_sidebar_folders();
        } else {
            let _ = app.queue_pending_sidebar_space_action(DOUBLE_SPACE_WINDOW);
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Esc => {}
        KeyCode::Char('j') | KeyCode::Down => app.next(),
        KeyCode::Char('k') | KeyCode::Up => app.previous(),
        KeyCode::Char('h') | KeyCode::Left => {
            let _ = app.collapse_selected_folder();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let _ = app.expand_selected_folder();
        }
        KeyCode::Char('d') => {
            if let Some(panel) = app.selected_panel() {
                app.sidebar.delete_target = Some(panel.clone());
                app.mode = Mode::DeleteConfirm;
                app.dirty = true;
            }
        }
        KeyCode::Char(' ') => {
            if app.sidebar.show_tree {
                if let Some(ref mut tree) = app.sidebar.file_tree {
                    tree.toggle();
                }
                app.dirty = true;
            } else {
                match app.selected_sidebar_item() {
                    Some(item) if item.as_folder().is_some() => {
                        let _ = app.toggle_selected_folder();
                    }
                    Some(item) if item.as_thread().is_some() => {
                        let _ = app.collapse_parent_folder_for_selected_thread();
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Enter => match app.selected_sidebar_item() {
            Some(item) if item.as_folder().is_some() => {
                let _ = app.toggle_selected_folder();
            }
            Some(item) if item.as_thread().is_some() => {
                if app
                    .selected_preview_thread()
                    .is_some_and(|thread| thread.is_live())
                {
                    attach_fn(terminal, app)?;
                } else {
                    app.invalidate_preview();
                    app.dirty = true;
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

fn handle_preview_tab(app: &mut App) {
    const DOUBLE_TAB_WINDOW: Duration = Duration::from_millis(350);

    if app.preview_is_focused() {
        if app.preview.view == crate::model::PreviewView::SessionDetail {
            app.note_detail_exit_tab();
            app.toggle_preview_focus();
            app.clear_panel_tab();
            return;
        }
        if app.recent_panel_tab_within(DOUBLE_TAB_WINDOW) && app.open_latest_preview_turn() {
            app.clear_panel_tab();
            return;
        }
        app.toggle_preview_focus();
        app.clear_panel_tab();
        return;
    }

    if app.recent_detail_exit_tab_within(DOUBLE_TAB_WINDOW)
        && app.selected_thread_matches_preview_target()
        && app.restore_preview_turns_list()
    {
        app.focus_panel();
        app.clear_detail_exit_tab();
        app.clear_panel_tab();
        return;
    }

    if app.toggle_preview_focus() {
        app.note_panel_tab();
        app.clear_detail_exit_tab();
    } else {
        app.clear_panel_tab();
        app.clear_detail_exit_tab();
    }
}

#[cfg_attr(test, allow(dead_code))]
fn handle_attach(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    attach::handle_attach(terminal, app)
}

fn handle_fuzzy_picker_mode(app: &mut App, key: crossterm::event::KeyEvent) {
    modes::handle_fuzzy_picker_mode(app, key);
}

fn handle_relay_settings_mode(app: &mut App, key: KeyCode) {
    modes::handle_relay_settings_mode(app, key);
}

fn handle_search_mode(app: &mut App, key: KeyCode) {
    modes::handle_search_mode(app, key);
}

fn handle_settings_mode(app: &mut App, key: KeyCode) {
    modes::handle_settings_mode(app, key);
}

fn handle_theme_selector_mode(app: &mut App, key: KeyCode) {
    modes::handle_theme_selector_mode(app, key);
}

fn handle_language_selector_mode(app: &mut App, key: KeyCode) {
    modes::handle_language_selector_mode(app, key);
}

fn handle_tree_mode(app: &mut App, key: KeyCode) {
    modes::handle_tree_mode(app, key);
}

fn handle_file_preview_mode(app: &mut App, key: KeyCode) {
    modes::handle_file_preview_mode(app, key);
}

fn handle_tree_search_mode(app: &mut App, key: KeyCode) {
    modes::handle_tree_search_mode(app, key);
}

fn handle_agent_launcher_mode(app: &mut App, key: KeyCode) {
    modes::handle_agent_launcher_mode(app, key);
}

fn handle_delete_confirm_mode(app: &mut App, key: KeyCode) {
    modes::handle_delete_confirm_mode(app, key);
}

fn handle_thread_action_confirm_mode(app: &mut App, key: KeyEvent) {
    modes::handle_thread_action_confirm_mode(app, key);
}

fn handle_help_mode(app: &mut App, key: KeyCode) {
    modes::handle_help_mode(app, key);
}

fn handle_agent_style_mode(app: &mut App, key: KeyCode) {
    modes::handle_agent_style_mode(app, key);
}

fn handle_telegram_settings_mode(app: &mut App, key: KeyCode) {
    modes::handle_telegram_settings_mode(app, key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::FocusTarget;
    use crate::model::{
        AgentPanel, AgentState, AgentStateSource, AgentType, PreviewSource, PreviewTurn,
        PreviewView,
    };
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
    use ratatui::backend::TestBackend;

    fn sample_panel(pane_id: &str, working_dir: &str) -> AgentPanel {
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

    fn test_terminal() -> ratatui::Terminal<TestBackend> {
        ratatui::Terminal::new(TestBackend::new(100, 20)).unwrap()
    }

    fn left_click(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn scroll_down(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

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
}
