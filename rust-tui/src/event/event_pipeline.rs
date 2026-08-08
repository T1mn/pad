use crate::app::App;
use crate::event::loop_state::LoopState;
use crossterm::event::{self, Event};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

pub(super) enum EventOutcome {
    Consumed,
    Processed,
}

pub(super) fn next_event(state: &mut LoopState, timeout: Duration) -> io::Result<Option<Event>> {
    if let Some(ev) = state.carried_event.take() {
        Ok(Some(ev))
    } else if crossterm::event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

pub(super) fn handle_event(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    state: &mut LoopState,
    ev: Event,
) -> io::Result<EventOutcome> {
    if let Event::Mouse(mouse) = ev {
        if super::mouse_pipeline::preprocess_scroll_burst(terminal, app, state, mouse)? {
            return Ok(EventOutcome::Consumed);
        }
    }

    match ev {
        Event::Key(key) => super::key_pipeline::handle_key_event(terminal, app, state, key)?,
        Event::Mouse(mouse) => super::mouse_pipeline::handle_mouse_event(terminal, app, mouse)?,
        Event::Resize(_, _) => {
            terminal.clear()?;
            app.dirty = true;
        }
        Event::Paste(text) => super::key_pipeline::handle_paste(app, &text),
        _ => {}
    }

    Ok(EventOutcome::Processed)
}
