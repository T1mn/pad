mod draw;
mod events;

use crate::app::App;
use crate::event::loop_state::LoopState;
use crate::pipe::TmuxEvent;
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
    app.flush_pending_sidebar_space_action_if_due();

    if app.refresh_after_attach {
        app.refresh_after_attach = false;
        app.refresh_panels();
        app.invalidate_preview();
    }

    check_async_results(app);
    check_preview_detail_update(terminal, app)?;

    events::drain_hook_events(app);
    events::drain_pipe_events(state, pipe_rx);
    events::trigger_debounced_scan(app, state);
    draw::clear_and_draw(terminal, app)?;

    Ok(())
}

pub(super) fn run_tick_cycle(app: &mut App, state: &mut LoopState, tick_rate: Duration) {
    if state.last_tick.elapsed() >= tick_rate {
        app.poll_external_relay_config_if_due();
        app.apply_pending_external_relay_reload_if_ready();
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

fn check_async_results(app: &mut App) {
    app.check_scan_result();
    app.check_preview_result();
    app.check_preview_detail_result();
    app.check_delayed_scan();
    app.check_provider_test_result();
    app.check_codex_cli_version_result();
    app.check_codex_cli_update_result();
    app.check_title_summary_result();
    app.check_preview_update();
}

fn check_preview_detail_update(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    if app.preview.view == crate::model::PreviewView::SessionDetail {
        let terminal_area: Rect = terminal.size()?.into();
        let preview_detail_width = super::mouse::normal_mouse_regions(app, terminal_area)
            .preview_content_area
            .width;
        app.check_preview_detail_update(preview_detail_width);
    }
    Ok(())
}
