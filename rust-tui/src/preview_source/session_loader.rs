mod cache;
mod continuity;
mod errors;
mod load;
mod logging;
mod parse;
mod persist;
mod resolved;

use crate::model::{PreviewSessionOrigin, SessionCacheState, SharedPreviewTurns};

pub(super) use load::load_session_preview;

pub(super) struct SessionPreviewData {
    pub(super) turns: SharedPreviewTurns,
    pub(super) session_origin: PreviewSessionOrigin,
    pub(super) session_id: Option<String>,
    pub(super) transcript_path: Option<String>,
    pub(super) cache_state: SessionCacheState,
    pub(super) updated_at: Option<i64>,
}
