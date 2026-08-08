mod load;
mod model {
    use crate::model::{
        AgentState, AgentType, PreviewSessionOrigin, PreviewSource, SessionCacheState,
        SharedPreviewTurns,
    };

    #[derive(Clone, Debug)]
    pub struct PreviewRequest {
        pub target_key: String,
        pub live_pane_id: Option<String>,
        pub agent_type: AgentType,
        pub working_dir: String,
        pub state: AgentState,
        pub transcript_path: Option<String>,
        pub cached_preview_turns: SharedPreviewTurns,
        pub session_cache_state: Option<SessionCacheState>,
        pub agent_session_id: Option<String>,
        pub session_origin: Option<PreviewSessionOrigin>,
        pub persist_resolved_session: bool,
        pub known_updated_at: Option<i64>,
    }

    #[derive(Clone, Debug)]
    pub struct PreviewUpdate {
        pub target_key: String,
        pub live_pane_id: Option<String>,
        pub content: String,
        pub source: PreviewSource,
        pub session_origin: Option<PreviewSessionOrigin>,
        pub session_id: Option<String>,
        pub turns: SharedPreviewTurns,
        pub transcript_path: Option<String>,
        pub session_cache_state: Option<SessionCacheState>,
        pub updated_at: Option<i64>,
    }
}
mod refresh {
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
}
mod tmux {
    use super::model::PreviewRequest;

    const TMUX_CAPTURE_LINES: usize = 50;

    pub(super) fn load_tmux_preview(request: &PreviewRequest) -> String {
        let Some(pane_id) = request.live_pane_id.as_deref() else {
            return String::from("No live pane available");
        };

        match crate::pty::capture_pane(pane_id, TMUX_CAPTURE_LINES) {
            Ok(content) => content,
            Err(_) => String::from("Failed to capture pane"),
        }
    }
}

pub use load::load_preview;
pub use model::{PreviewRequest, PreviewUpdate};
pub use refresh::preview_refresh_interval_ms_for_request;
