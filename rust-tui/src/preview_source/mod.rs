mod claude;
pub(crate) mod codex;
mod gemini;
mod opencode;
mod session_loader;
mod session_target;
mod turns;

use crate::i18n::Locale;
use crate::model::{
    AgentPanel, AgentState, AgentType, PreviewSessionOrigin, PreviewSource, SessionCacheState,
    SharedPreviewTurns,
};

const TMUX_CAPTURE_LINES: usize = 50;
const BUSY_REFRESH_MS: u64 = 1000;
const WAITING_REFRESH_MS: u64 = 1200;
const APP_IDLE_REFRESH_MS: u64 = 1200;
const LIVE_IDLE_REFRESH_MS: u64 = 2500;
const HISTORY_IDLE_REFRESH_MS: u64 = 4000;

use session_loader::load_session_preview;

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

#[derive(Clone, Copy)]
pub(super) enum SessionReadMode {
    FullBackfill,
}

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

pub fn load_preview(request: &PreviewRequest, mode: &str, locale: Locale) -> PreviewUpdate {
    let preferred_source = resolve_preferred_source(request, mode);
    let (
        content,
        source,
        session_origin,
        session_id,
        turns,
        transcript_path,
        session_cache_state,
        updated_at,
    ) = match preferred_source {
        PreviewSource::Tmux => (
            load_tmux_preview(request),
            PreviewSource::Tmux,
            None,
            None,
            SharedPreviewTurns::default(),
            None,
            None,
            None,
        ),
        PreviewSource::Session => match load_session_preview(request, locale) {
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
            Err(_err) if mode == "auto" => (
                load_tmux_preview(request),
                PreviewSource::Tmux,
                None,
                None,
                SharedPreviewTurns::default(),
                None,
                None,
                None,
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
        },
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

fn resolve_preferred_source(request: &PreviewRequest, mode: &str) -> PreviewSource {
    match mode {
        "tmux" => PreviewSource::Tmux,
        "session" => PreviewSource::Session,
        _ => {
            if supports_session_preview(request) {
                PreviewSource::Session
            } else {
                PreviewSource::Tmux
            }
        }
    }
}

fn supports_session_preview(request: &PreviewRequest) -> bool {
    match request.agent_type {
        AgentType::Codex => true,
        AgentType::Claude => {
            request.transcript_path.is_some()
                || request.agent_session_id.is_some()
                || !request.cached_preview_turns.is_empty()
        }
        AgentType::Gemini | AgentType::OpenCode => true,
        _ => false,
    }
}

fn load_tmux_preview(request: &PreviewRequest) -> String {
    let Some(pane_id) = request.live_pane_id.as_deref() else {
        return String::from("No live pane available");
    };

    match crate::pty::capture_pane(pane_id, TMUX_CAPTURE_LINES) {
        Ok(content) => content,
        Err(_) => String::from("Failed to capture pane"),
    }
}

#[cfg(test)]
mod tests;
