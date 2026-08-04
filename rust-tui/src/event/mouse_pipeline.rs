use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::MouseEvent;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub(super) fn preprocess_scroll_burst(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    state: &mut super::loop_state::LoopState,
    mouse: MouseEvent,
) -> io::Result<bool> {
    if app.mode == Mode::Normal && app.terminal_is_active() {
        let regions = super::mouse::normal_mouse_regions(app, terminal.size()?.into());
        if regions
            .preview_inner
            .contains((mouse.column, mouse.row).into())
        {
            if app.terminal_is_focused() {
                if let Some(bytes) = crate::terminal_runtime::encode_mouse_event(
                    mouse,
                    regions.preview_inner,
                    app.terminal_mode(),
                ) {
                    let _ = app.send_terminal_input(bytes);
                }
            }
            // Scrollback ownership belongs to the terminal even when the
            // child has not enabled mouse reporting; preview scrolling must
            // never run underneath it.
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

pub(super) fn handle_mouse_event(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mouse: MouseEvent,
) -> io::Result<()> {
    if app.mode == Mode::Normal {
        if app.terminal_is_active() {
            let regions = super::mouse::normal_mouse_regions(app, terminal.size()?.into());
            if regions
                .preview_area
                .contains((mouse.column, mouse.row).into())
            {
                if matches!(mouse.kind, crossterm::event::MouseEventKind::Down(_)) {
                    app.focus_terminal();
                }
                if app.terminal_is_focused() {
                    if let Some(bytes) = crate::terminal_runtime::encode_mouse_event(
                        mouse,
                        regions.preview_inner,
                        app.terminal_mode(),
                    ) {
                        let _ = app.send_terminal_input(bytes);
                    }
                }
                return Ok(());
            }
        }
        super::mouse::handle_normal_mouse(app, terminal.size()?.into(), mouse);
    }
    Ok(())
}
