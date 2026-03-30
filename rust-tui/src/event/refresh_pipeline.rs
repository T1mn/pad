use crate::app::App;
use crate::event::loop_state::LoopState;
use crate::log_debug;
use crate::pipe::TmuxEvent;
use crate::ui;
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::io;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub(super) fn run_pre_event_cycle(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    state: &mut LoopState,
    pipe_rx: &mut mpsc::Receiver<TmuxEvent>,
) -> io::Result<()> {
    app.flush_deferred_ui_updates();
    app.expire_copy_toast_if_needed();

    if app.refresh_after_attach {
        app.refresh_after_attach = false;
        app.refresh_panels();
        app.invalidate_preview();
    }

    app.check_scan_result();
    app.check_preview_result();
    app.check_preview_detail_result();
    app.check_delayed_scan();
    app.check_provider_test_result();
    app.check_preview_update();
    let terminal_area: Rect = terminal.size()?.into();
    let preview_detail_width = super::mouse::normal_mouse_regions(app, terminal_area)
        .preview_content_area
        .width;
    app.check_preview_detail_update(preview_detail_width);

    drain_hook_events(app);
    drain_pipe_events(state, pipe_rx);
    trigger_debounced_scan(app, state);
    clear_and_draw(terminal, app)?;

    Ok(())
}

pub(super) fn run_tick_cycle(app: &mut App, state: &mut LoopState, tick_rate: Duration) {
    if state.last_tick.elapsed() >= tick_rate {
        if app.should_tick_busy_animation() {
            app.busy_animation_frame = app.busy_animation_frame.wrapping_add(1);
            app.last_busy_animation_tick = Instant::now();
            app.dirty = true;
        }
        // Keep interval polling even when control-mode pipe is active.
        // This preserves scanner-driven status refresh without relying on
        // noisy tmux %output notifications.
        if app.config.auto_refresh
            && app.last_refresh.elapsed()
                >= std::time::Duration::from_secs(app.config.refresh_interval)
            && !app.scan_in_progress
        {
            app.trigger_async_scan();
        }
        state.last_tick = Instant::now();
    }
}

fn drain_hook_events(app: &mut App) {
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

fn drain_pipe_events(state: &mut LoopState, pipe_rx: &mut mpsc::Receiver<TmuxEvent>) {
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

fn trigger_debounced_scan(app: &mut App, state: &mut LoopState) {
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

fn clear_and_draw(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    if app.needs_clear {
        terminal.clear()?;
        app.needs_clear = false;
        app.dirty = true;
    }

    if app.dirty {
        let draw_started = Instant::now();
        terminal.draw(|f| ui::draw(f, app))?;
        app.last_draw_elapsed = draw_started.elapsed();
        app.frame_budget_exceeded = app.last_draw_elapsed >= Duration::from_millis(12);
        if app.frame_budget_exceeded {
            log_debug!(
                "ui.frame: draw_slow elapsed_ms={} detail={} dirty_sidebar={}/{}",
                app.last_draw_elapsed.as_millis(),
                app.preview.view == crate::model::PreviewView::SessionDetail,
                app.sidebar.sidebar_folders_dirty,
                app.sidebar.visible_sidebar_items_dirty
            );
        }
        app.dirty = false;
    }

    Ok(())
}
