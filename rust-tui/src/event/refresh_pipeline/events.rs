use crate::app::App;
use crate::event::loop_state::LoopState;
use crate::log_debug;
use crate::pipe::TmuxEvent;
use std::time::Instant;
use tokio::sync::mpsc;

pub(super) fn drain_hook_events(app: &mut App) {
    let mut pending_hook_events = Vec::new();
    if let Some(ref mut hook_rx) = app.hook_rx {
        loop {
            match hook_rx.try_recv() {
                Ok(ev) => pending_hook_events.push(ev),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    app.hook_rx = None;
                    break;
                }
            }
        }
    }
    for ev in pending_hook_events {
        if app.should_defer_ui_updates() {
            app.deferred_hook_events.push(ev);
        } else {
            app.apply_hook_event(ev);
        }
    }
}

pub(super) fn drain_pipe_events(state: &mut LoopState, pipe_rx: &mut mpsc::Receiver<TmuxEvent>) {
    loop {
        match pipe_rx.try_recv() {
            Ok(ev) => match ev {
                TmuxEvent::WindowChanged | TmuxEvent::SessionChanged => {
                    state.pipe_fast_pending = true;
                    state.last_pipe_fast = Instant::now();
                }
                TmuxEvent::PaneModeChanged | TmuxEvent::OutputDetected => {
                    state.pipe_slow_pending = true;
                    state.last_pipe_slow = Instant::now();
                }
                TmuxEvent::Disconnected => {
                    log_debug!("pipe: disconnected");
                }
            },
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
        }
    }
}

pub(super) fn trigger_debounced_scan(app: &mut App, state: &mut LoopState) {
    let should_scan = (state.pipe_fast_pending
        && state.last_pipe_fast.elapsed() >= state.debounce_fast)
        || (state.pipe_slow_pending && state.last_pipe_slow.elapsed() >= state.debounce_slow);
    if should_scan && !app.scan_in_progress {
        state.pipe_fast_pending = false;
        state.pipe_slow_pending = false;
        log_debug!("pipe: triggering scan");
        app.trigger_async_scan();
    }
}
