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
    let mut pipe_rx = crate::pipe::start_control_pipe();

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
