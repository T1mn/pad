use crate::app::App;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[cfg_attr(test, allow(dead_code))]
mod attach;
mod event_pipeline;
mod key_pipeline;
mod loop_core;
mod loop_state;
mod mode_dispatch;
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
