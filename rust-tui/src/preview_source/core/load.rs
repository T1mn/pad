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
