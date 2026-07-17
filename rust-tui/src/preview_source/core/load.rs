use super::model::{PreviewRequest, PreviewUpdate};
use super::tmux::load_tmux_preview;
use crate::i18n::Locale;
use crate::model::{AgentType, PreviewSource, SharedPreviewTurns};
use crate::preview_source::session_loader::load_session_preview;

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
        AgentType::Gemini | AgentType::Grok | AgentType::OpenCode => true,
        _ => false,
    }
}
