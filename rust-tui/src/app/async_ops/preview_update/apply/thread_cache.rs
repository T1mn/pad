use crate::app::{App, ThreadPreviewCacheEntry};
use crate::model::PreviewSource;
use crate::preview_source::PreviewUpdate;

pub(super) fn update_thread_preview_cache(app: &mut App, update: &PreviewUpdate) {
    if update.source != PreviewSource::Session || update.turns.is_empty() {
        return;
    }

    let previous_updated_at = app
        .preview
        .thread_preview_cache
        .get(&update.target_key)
        .and_then(|entry| entry.updated_at);
    app.preview.thread_preview_cache.insert(
        update.target_key.clone(),
        ThreadPreviewCacheEntry {
            turns: update.turns.clone(),
            session_cache_state: update.session_cache_state,
            transcript_path: update.transcript_path.clone(),
            session_id: update.session_id.clone(),
            updated_at: update.updated_at,
            cached_at: crate::app::unix_now_ts(),
        },
    );
    let preview_cache_pruned = app.prune_thread_preview_cache();
    if update.updated_at != previous_updated_at || preview_cache_pruned {
        app.invalidate_sidebar_cache();
    }
}
