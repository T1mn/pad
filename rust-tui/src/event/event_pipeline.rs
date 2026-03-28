use crate::app::App;
use crate::event::loop_state::LoopState;
use crate::log_debug;
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

    maybe_refresh_after_same_session_return(app, &ev);

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

fn maybe_refresh_after_same_session_return(app: &mut App, ev: &Event) {
    // If we returned from same-session attach (F12/C-q self-restored the bindings
    // in tmux), refresh only after tmux actually switches the active pane back to pad.
    // This avoids treating the initial FocusLost from switching away as a return signal.
    if app.same_session_attached {
        let focus_state = super::pad_focus_state();
        if let Some((pad_pane_id, current_pane_id)) = focus_state {
            if current_pane_id == pad_pane_id {
                log_debug!(
                    "same_session_attached: pad pane active via {:?}, current_pane={} pad_pane={}, refreshing",
                    ev,
                    current_pane_id,
                    pad_pane_id
                );
                app.same_session_attached = false;
                app.saved_tmux_bindings.clear();
                app.saved_tmux_status = None;
                app.saved_tmux_status_target = None;
                app.refresh_panels();
                app.invalidate_preview();
                app.needs_clear = true;
                app.dirty = true;
            } else if matches!(
                ev,
                Event::FocusLost | Event::FocusGained | Event::Resize(_, _)
            ) {
                log_debug!(
                    "same_session_attached: waiting return event={:?} current_pane={} pad_pane={}",
                    ev,
                    current_pane_id,
                    pad_pane_id
                );
            }
        } else if matches!(
            ev,
            Event::FocusLost | Event::FocusGained | Event::Resize(_, _)
        ) {
            log_debug!(
                "same_session_attached: waiting return event={:?} focus_state=unknown",
                ev
            );
        }
    }
}
