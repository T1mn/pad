mod claude;
pub(crate) mod codex;
mod core;
mod gemini;
mod opencode;
mod session_loader;
mod session_target;
mod turns;

pub use core::{
    load_preview, preview_refresh_interval_ms_for_request, PreviewRequest, PreviewUpdate,
};

#[allow(dead_code)]
pub fn preview_refresh_interval_ms(panel: &crate::model::AgentPanel) -> u64 {
    core::preview_refresh_interval_ms(panel)
}

#[allow(dead_code)]
pub fn preview_refresh_interval_ms_for_state(state: &crate::model::AgentState) -> u64 {
    core::preview_refresh_interval_ms_for_state(state)
}

#[derive(Clone, Copy)]
pub(super) enum SessionReadMode {
    FullBackfill,
}

#[cfg(test)]
mod tests;
