use crate::app::state::Mode;
use crate::app::App;
use crate::terminal_runtime::{TerminalMode, TerminalScroll};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

const TERMINAL_SCROLL_LINES_PER_TICK: i32 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalWheelRoute {
    Child,
    Scroll(TerminalScroll),
}

pub(super) fn preprocess_scroll_burst(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    state: &mut super::loop_state::LoopState,
    mouse: MouseEvent,
) -> io::Result<bool> {
    let Some(_) = super::mouse::mouse_scroll_delta(&mouse.kind) else {
        return Ok(false);
    };

    if app.mode == Mode::Normal && app.terminal_is_active() && !app.sidebar.show_tree {
        let regions = super::mouse::normal_mouse_regions(app, terminal.size()?.into());
        if regions
            .preview_area
            .contains((mouse.column, mouse.row).into())
        {
            let placement = crate::ui::terminal::placement(app, regions.preview_area);
            let pane = placement
                .panes
                .iter()
                .find(|pane| pane.inner.contains((mouse.column, mouse.row).into()))
                .map(|pane| (pane.pane_id, pane.inner));
            let Some((pane_id, pane_inner)) = pane else {
                // The terminal owns its tab row and pane borders; do not let
                // preview scrolling run underneath those areas.
                return Ok(true);
            };
            app.focus_terminal();
            if let Err(error) = app.focus_terminal_pane(pane_id) {
                app.show_action_toast("PAD Terminal", &error.to_string());
            }
            let mode = app
                .terminal_pane(pane_id)
                .map(|pane| pane.mode())
                .unwrap_or_default();
            match terminal_wheel_route(&mouse, mode) {
                Some(TerminalWheelRoute::Child) => {
                    if let Some(bytes) =
                        crate::terminal_runtime::encode_mouse_event(mouse, pane_inner, mode)
                    {
                        let _ = app.send_terminal_mouse_input(bytes);
                    }
                }
                Some(TerminalWheelRoute::Scroll(_)) => {
                    let (_, _, delta) =
                        super::mouse::coalesce_scroll_burst(mouse, &mut state.carried_event)?;
                    let _ = app.scroll_terminal(TerminalScroll::Lines(
                        -delta * TERMINAL_SCROLL_LINES_PER_TICK,
                    ));
                }
                None => {}
            }
            return Ok(true);
        }
    }
    if super::mouse::mouse_scroll_delta(&mouse.kind).is_some() {
        let (column, row, delta) =
            super::mouse::coalesce_scroll_burst(mouse, &mut state.carried_event)?;
        if app.mode == Mode::Normal {
            app.clear_panel_tab();
            super::mouse::handle_normal_scroll(app, terminal.size()?.into(), column, row, delta);
        }
        return Ok(true);
    }
    Ok(false)
}

fn terminal_wheel_route(mouse: &MouseEvent, mode: TerminalMode) -> Option<TerminalWheelRoute> {
    let delta = super::mouse::mouse_scroll_delta(&mouse.kind)?;
    if mode.mouse_reporting && !mouse.modifiers.contains(KeyModifiers::SHIFT) {
        Some(TerminalWheelRoute::Child)
    } else {
        Some(TerminalWheelRoute::Scroll(TerminalScroll::Lines(
            -delta * TERMINAL_SCROLL_LINES_PER_TICK,
        )))
    }
}

pub(super) fn handle_mouse_event(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mouse: MouseEvent,
) -> io::Result<()> {
    if app.mode == Mode::Normal {
        if app.terminal_is_active() && !app.sidebar.show_tree {
            let regions = super::mouse::normal_mouse_regions(app, terminal.size()?.into());
            if regions
                .preview_area
                .contains((mouse.column, mouse.row).into())
            {
                let placement = crate::ui::terminal::placement(app, regions.preview_area);
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                    && placement.tab_bar.contains((mouse.column, mouse.row).into())
                {
                    app.focus_terminal();
                    if let Some(tab) = placement
                        .tabs
                        .iter()
                        .find(|tab| tab.rect.contains((mouse.column, mouse.row).into()))
                    {
                        if let Err(error) = app.focus_terminal_tab(tab.index) {
                            app.show_action_toast("PAD Terminal", &error.to_string());
                        }
                    }
                    return Ok(());
                }

                let pane = placement
                    .panes
                    .iter()
                    .find(|pane| pane.outer.contains((mouse.column, mouse.row).into()))
                    .map(|pane| (pane.pane_id, pane.inner));
                let Some((pane_id, pane_inner)) = pane else {
                    return Ok(());
                };
                if matches!(mouse.kind, MouseEventKind::Down(_)) {
                    app.focus_terminal();
                    if let Err(error) = app.focus_terminal_pane(pane_id) {
                        app.show_action_toast("PAD Terminal", &error.to_string());
                    }
                }
                if app.terminal_is_focused()
                    && app.focused_terminal_pane_id() == Some(pane_id)
                    && pane_inner.contains((mouse.column, mouse.row).into())
                {
                    let mode = app
                        .terminal_pane(pane_id)
                        .map(|pane| pane.mode())
                        .unwrap_or_default();
                    if let Some(bytes) =
                        crate::terminal_runtime::encode_mouse_event(mouse, pane_inner, mode)
                    {
                        let _ = app.send_terminal_mouse_input(bytes);
                    }
                }
                return Ok(());
            }
        }
        super::mouse::handle_normal_mouse(app, terminal.size()?.into(), mouse);
    }
    Ok(())
}

#[cfg(test)]
#[path = "mouse_pipeline_tests.rs"]
mod tests;
