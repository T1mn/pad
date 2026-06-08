use super::model::PreviewRequest;
use crate::model::{AgentPanel, AgentState, PreviewSessionOrigin};

const BUSY_REFRESH_MS: u64 = 1000;
const WAITING_REFRESH_MS: u64 = 1200;
const APP_IDLE_REFRESH_MS: u64 = 1200;
const LIVE_IDLE_REFRESH_MS: u64 = 2500;
const HISTORY_IDLE_REFRESH_MS: u64 = 4000;

#[allow(dead_code)]
pub fn preview_refresh_interval_ms(panel: &AgentPanel) -> u64 {
    match panel.state {
        AgentState::Busy => BUSY_REFRESH_MS,
        AgentState::Waiting => WAITING_REFRESH_MS,
        AgentState::Idle => LIVE_IDLE_REFRESH_MS,
    }
}

#[allow(dead_code)]
pub fn preview_refresh_interval_ms_for_state(state: &AgentState) -> u64 {
    match state {
        AgentState::Busy => BUSY_REFRESH_MS,
        AgentState::Waiting => WAITING_REFRESH_MS,
        AgentState::Idle => LIVE_IDLE_REFRESH_MS,
    }
}

pub fn preview_refresh_interval_ms_for_request(request: &PreviewRequest) -> u64 {
    match request.state {
        AgentState::Busy => BUSY_REFRESH_MS,
        AgentState::Waiting => WAITING_REFRESH_MS,
        AgentState::Idle => match request.session_origin {
            Some(PreviewSessionOrigin::App) => APP_IDLE_REFRESH_MS,
            _ if request.live_pane_id.is_some() => LIVE_IDLE_REFRESH_MS,
            _ => HISTORY_IDLE_REFRESH_MS,
        },
    }
}
