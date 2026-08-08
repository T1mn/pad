use crate::app::App;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[cfg_attr(test, allow(dead_code))]
mod attach;
mod event_pipeline;
mod input_clear;
mod key_pipeline;
mod loop_core {
    use crate::app::App;
    use crate::log_debug;
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;
    use std::thread;
    use std::time::Duration;

    pub(super) async fn run_app(
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        app: &mut App,
    ) -> io::Result<()> {
        let mut state = super::loop_state::LoopState::new();
        let (disabled_pipe_tx, disabled_pipe_rx) = tokio::sync::mpsc::channel(1);
        let _disabled_pipe_tx = (!app.runtime_mode.uses_tmux()).then_some(disabled_pipe_tx);
        let mut pipe_rx = if app.runtime_mode.uses_tmux() {
            crate::pipe::start_control_pipe()
        } else {
            disabled_pipe_rx
        };

        loop {
            super::refresh_pipeline::run_pre_event_cycle(terminal, app, &mut state, &mut pipe_rx)?;
            let tick_rate = app.desired_tick_rate();
            let timeout = state.timeout(tick_rate);
            let next_event = match super::event_pipeline::next_event(&mut state, timeout) {
                Ok(next_event) => next_event,
                Err(err) if should_wait_for_hidden_pad(app) => {
                    let trace_id = app.same_session_trace_id.as_deref().unwrap_or("-");
                    log_debug!(
                        "handoff trace={} stage=event_loop.hidden_input_error suppressed=true err={}",
                        trace_id,
                        err
                    );
                    thread::sleep(Duration::from_millis(50));
                    None
                }
                Err(err) => {
                    let trace_id = app.same_session_trace_id.as_deref().unwrap_or("-");
                    log_debug!(
                        "handoff trace={} stage=event_loop.input_error suppressed=false err={}",
                        trace_id,
                        err
                    );
                    return Err(err);
                }
            };
            if let Some(ev) = next_event {
                let outcome = super::event_pipeline::handle_event(terminal, app, &mut state, ev)?;
                if matches!(outcome, super::event_pipeline::EventOutcome::Consumed) {
                    continue;
                }
            }

            super::refresh_pipeline::run_tick_cycle(app, &mut state, tick_rate);
            if app.should_quit {
                return Ok(());
            }
        }
    }

    fn should_wait_for_hidden_pad(app: &App) -> bool {
        app.same_session_attached
            && super::pad_focus_state()
                .map(|(pad_pane_id, current_pane_id)| current_pane_id != pad_pane_id)
                .unwrap_or(true)
    }
}
mod loop_state {
    use crossterm::event::Event;
    use std::time::{Duration, Instant};

    pub(super) struct LoopState {
        pub(super) last_tick: Instant,
        pub(super) carried_event: Option<Event>,
        pub(super) pipe_fast_pending: bool,
        pub(super) pipe_slow_pending: bool,
        pub(super) last_pipe_fast: Instant,
        pub(super) last_pipe_slow: Instant,
        pub(super) debounce_fast: Duration,
        pub(super) debounce_slow: Duration,
    }

    impl LoopState {
        pub(super) fn new() -> Self {
            let now = Instant::now();
            Self {
                last_tick: now,
                carried_event: None,
                pipe_fast_pending: false,
                pipe_slow_pending: false,
                last_pipe_fast: now,
                last_pipe_slow: now,
                debounce_fast: Duration::from_millis(100),
                debounce_slow: Duration::from_millis(500),
            }
        }

        pub(super) fn timeout(&self, tick_rate: Duration) -> Duration {
            tick_rate
                .checked_sub(self.last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0))
        }
    }
}
mod mode_dispatch {
    use super::modes;
    use crate::app::App;
    use crossterm::event::{KeyCode, KeyEvent};

    pub(super) fn handle_fuzzy_picker_mode(app: &mut App, key: crossterm::event::KeyEvent) {
        modes::handle_fuzzy_picker_mode(app, key);
    }

    pub(super) fn handle_relay_settings_mode(app: &mut App, key: KeyCode) {
        modes::handle_relay_settings_mode(app, key);
    }

    pub(super) fn handle_search_mode(app: &mut App, key: KeyCode) {
        modes::handle_search_mode(app, key);
    }

    pub(super) fn handle_settings_mode(app: &mut App, key: KeyCode) {
        modes::handle_settings_mode(app, key);
    }

    pub(super) fn handle_tree_mode(app: &mut App, key: KeyCode) {
        modes::handle_tree_mode(app, key);
    }

    pub(super) fn handle_file_preview_mode(app: &mut App, key: KeyCode) {
        modes::handle_file_preview_mode(app, key);
    }

    pub(super) fn handle_tree_search_mode(app: &mut App, key: KeyCode) {
        modes::handle_tree_search_mode(app, key);
    }

    pub(super) fn handle_agent_launcher_mode(app: &mut App, key: KeyCode) {
        modes::handle_agent_launcher_mode(app, key);
    }

    pub(super) fn handle_delete_confirm_mode(app: &mut App, key: KeyCode) {
        modes::handle_delete_confirm_mode(app, key);
    }

    pub(super) fn handle_thread_action_confirm_mode(app: &mut App, key: KeyEvent) {
        modes::handle_thread_action_confirm_mode(app, key);
    }

    pub(super) fn handle_help_mode(app: &mut App, key: KeyCode) {
        modes::handle_help_mode(app, key);
    }

    pub(super) fn handle_agent_style_mode(app: &mut App, key: KeyCode) {
        modes::handle_agent_style_mode(app, key);
    }

    pub(super) fn handle_telegram_settings_mode(app: &mut App, key: KeyCode) {
        modes::handle_telegram_settings_mode(app, key);
    }

    pub(super) fn handle_notification_inbox_mode(app: &mut App, key: KeyCode) {
        modes::handle_notification_inbox_mode(app, key);
    }
}
mod modes;
mod mouse;
mod mouse_pipeline;
mod normal;
mod refresh_pipeline;

#[cfg(test)]
mod tests;
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
