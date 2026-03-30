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
        super::mouse::handle_normal_mouse(app, terminal.size()?.into(), mouse);
    }
    Ok(())
}
