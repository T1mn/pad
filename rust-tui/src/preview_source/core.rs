mod load {
    use super::model::{PreviewRequest, PreviewUpdate};
    use crate::i18n::Locale;
    use crate::model::{PreviewSource, SharedPreviewTurns};
    use crate::preview_source::session_loader::load_session_preview;

    pub fn load_preview(request: &PreviewRequest, _mode: &str, locale: Locale) -> PreviewUpdate {
        let (
            content,
            source,
            session_origin,
            session_id,
            turns,
            transcript_path,
            session_cache_state,
            updated_at,
        ) = match load_session_preview(request, locale) {
            Ok(data) => (
                // Session UI renders from structured turns. Avoid building and
                // storing a second full transcript string on every preview tick.
                String::new(),
                PreviewSource::Session,
                Some(data.session_origin),
                data.session_id,
                data.turns,
                data.transcript_path,
                Some(data.cache_state),
                data.updated_at,
            ),
            Err(err) => (
                err,
                PreviewSource::Session,
                None,
                None,
                SharedPreviewTurns::default(),
                None,
                None,
                None,
            ),
        };

        PreviewUpdate {
            target_key: request.target_key.clone(),
            live_pane_id: request.live_pane_id.clone(),
            content,
            source,
            session_origin,
            session_id,
            turns,
            transcript_path,
            session_cache_state,
            updated_at,
        }
    }
}
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
pub use load::load_preview;
pub use model::{PreviewRequest, PreviewUpdate};
pub use refresh::preview_refresh_interval_ms_for_request;
