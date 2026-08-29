use crate::app::state::Mode;
use crate::app::App;
use crate::terminal_runtime::{TerminalMode, TerminalScroll};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

const TERMINAL_SCROLL_LINES_PER_TICK: i32 = 3;
const SIDEBAR_DIVIDER_HIT_SLOP: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalWheelRoute {
    Child,
    Scroll(TerminalScroll),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalTabPointerAction {
    Focus(usize),
    Close(usize),
}

pub(super) fn preprocess_scroll_burst(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    state: &mut super::loop_state::LoopState,
    mouse: MouseEvent,
) -> io::Result<bool> {
    if super::mouse::mouse_horizontal_scroll_delta(&mouse.kind).is_some() {
        if app.mode == Mode::Normal && app.terminal_is_active() && !app.sidebar.show_tree {
            let regions = super::mouse::normal_mouse_regions(app, terminal.size()?.into());
            if regions
                .preview_area
                .contains((mouse.column, mouse.row).into())
            {
                let (_, _, delta) = super::mouse::coalesce_horizontal_scroll_burst(
                    mouse,
                    &mut state.carried_event,
                )?;
                let direction = delta.signum() as isize;
                if direction != 0 {
                    app.focus_terminal();
                    if let Err(error) = app.cycle_terminal_tab(direction) {
                        app.show_action_toast("PAD Terminal", &error.to_string());
                    }
                }
                return Ok(true);
            }
        }
        return Ok(false);
    }
    let Some(_) = super::mouse::mouse_scroll_delta(&mouse.kind) else {
        return Ok(false);
    };

    if app.mode == Mode::Normal && app.terminal_is_active() && !app.sidebar.show_tree {
        let regions = super::mouse::normal_mouse_regions(app, terminal.size()?.into());
        if regions
            .preview_area
            .contains((mouse.column, mouse.row).into())
        {
            if app.terminal_workspace().panes.is_empty() {
                return Ok(true);
            }
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
        let terminal_area = terminal.size()?.into();
        if handle_sidebar_resize_mouse(app, terminal_area, mouse) {
            return Ok(());
        }
        if app.terminal_is_active() && !app.sidebar.show_tree {
            let regions = super::mouse::normal_mouse_regions(app, terminal_area);
            if regions
                .preview_area
                .contains((mouse.column, mouse.row).into())
            {
                if app.terminal_workspace().panes.is_empty() {
                    return Ok(());
                }
                let placement = crate::ui::terminal::placement(app, regions.preview_area);
                if placement.tab_bar.contains((mouse.column, mouse.row).into())
                    && matches!(
                        mouse.kind,
                        MouseEventKind::Down(MouseButton::Left | MouseButton::Middle)
                            | MouseEventKind::Drag(MouseButton::Left)
                    )
                {
                    match terminal_tab_pointer_action(&placement, mouse) {
                        Some(TerminalTabPointerAction::Focus(index)) => {
                            app.focus_terminal();
                            if let Err(error) = app.focus_terminal_tab(index) {
                                app.show_action_toast("PAD Terminal", &error.to_string());
                            }
                        }
                        Some(TerminalTabPointerAction::Close(index)) => {
                            if let Err(error) = app.close_terminal_tab(index) {
                                app.show_action_toast("PAD Terminal", &error.to_string());
                            }
                        }
                        None => {}
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
        super::mouse::handle_normal_mouse(app, terminal_area, mouse);
    }
    Ok(())
}

fn terminal_tab_pointer_action(
    placement: &crate::ui::terminal::TerminalPlacement,
    mouse: MouseEvent,
) -> Option<TerminalTabPointerAction> {
    let position = (mouse.column, mouse.row).into();
    let tab = placement
        .tabs
        .iter()
        .find(|tab| tab.rect.contains(position))?;
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Middle) => {
            Some(TerminalTabPointerAction::Close(tab.index))
        }
        MouseEventKind::Down(MouseButton::Left)
            if tab.close.is_some_and(|close| close.contains(position)) =>
        {
            Some(TerminalTabPointerAction::Close(tab.index))
        }
        MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {
            Some(TerminalTabPointerAction::Focus(tab.index))
        }
        _ => None,
    }
}

fn handle_sidebar_resize_mouse(
    app: &mut App,
    terminal_area: ratatui::layout::Rect,
    mouse: MouseEvent,
) -> bool {
    if app.sidebar.show_tree || app.sidebar.collapsed {
        app.sidebar.panel_resize_dragging = false;
        return false;
    }

    let regions = super::mouse::normal_mouse_regions(app, terminal_area);
    let divider = regions.preview_area.x;
    let inside_body = mouse.row >= regions.preview_area.y
        && mouse.row
            < regions
                .preview_area
                .y
                .saturating_add(regions.preview_area.height);
    let on_divider = inside_body
        && (mouse.column == divider
            || mouse.column.saturating_add(SIDEBAR_DIVIDER_HIT_SLOP) == divider);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) if on_divider => {
            app.sidebar.panel_resize_dragging = true;
            update_sidebar_width_from_mouse(app, terminal_area, mouse.column);
            true
        }
        MouseEventKind::Drag(MouseButton::Left) if app.sidebar.panel_resize_dragging => {
            update_sidebar_width_from_mouse(app, terminal_area, mouse.column);
            true
        }
        MouseEventKind::Up(MouseButton::Left) if app.sidebar.panel_resize_dragging => {
            update_sidebar_width_from_mouse(app, terminal_area, mouse.column);
            app.sidebar.panel_resize_dragging = false;
            app.commit_agent_panel_width();
            true
        }
        _ => false,
    }
}

fn update_sidebar_width_from_mouse(
    app: &mut App,
    terminal_area: ratatui::layout::Rect,
    column: u16,
) {
    let requested = column.saturating_sub(terminal_area.x);
    let width = crate::ui::layout::clamp_normal_left_width(terminal_area.width, requested);
    app.set_agent_panel_width(width);
}

#[cfg(test)]
#[path = "mouse_pipeline_tests.rs"]
pub(crate) mod tests;
