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
