use crate::app::App;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

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
        let next_event = super::event_pipeline::next_event(&mut state, timeout)?;
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
