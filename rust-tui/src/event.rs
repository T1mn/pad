use crate::app::App;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod event_pipeline {
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

    pub(super) fn next_event(
        state: &mut LoopState,
        timeout: Duration,
    ) -> io::Result<Option<Event>> {
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
}
pub(crate) mod input_clear;
mod key_pipeline;
mod loop_core {
    use crate::app::App;
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;

    pub(super) async fn run_app(
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        app: &mut App,
    ) -> io::Result<()> {
        let mut state = super::loop_state::LoopState::new();

        loop {
            super::refresh_pipeline::run_pre_event_cycle(terminal, app)?;
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
}
mod loop_state {
    use crossterm::event::Event;
    use std::time::{Duration, Instant};

    pub(super) struct LoopState {
        pub(super) last_tick: Instant,
        pub(super) carried_event: Option<Event>,
    }

    impl LoopState {
        pub(super) fn new() -> Self {
            let now = Instant::now();
            Self {
                last_tick: now,
                carried_event: None,
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

    pub(super) fn handle_telegram_settings_mode(app: &mut App, key: KeyCode) {
        modes::handle_telegram_settings_mode(app, key);
    }

    pub(super) fn handle_notification_inbox_mode(app: &mut App, key: KeyCode) {
        modes::handle_notification_inbox_mode(app, key);
    }
}
pub(crate) mod modes;
mod mouse;
pub(crate) mod mouse_pipeline;
pub(crate) mod normal;
mod refresh_pipeline;

#[cfg(test)]
pub(crate) mod tests;

pub async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop_core::run_app(terminal, app).await
}
