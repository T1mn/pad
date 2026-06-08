use super::model::PreviewRequest;
use crate::model::{AgentState, PreviewSessionOrigin};

const BUSY_REFRESH_MS: u64 = 1000;
const WAITING_REFRESH_MS: u64 = 1200;
const APP_IDLE_REFRESH_MS: u64 = 1200;
const LIVE_IDLE_REFRESH_MS: u64 = 2500;
const HISTORY_IDLE_REFRESH_MS: u64 = 4000;

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
