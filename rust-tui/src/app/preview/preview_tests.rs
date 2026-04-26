use crate::app::state::FocusTarget;
use crate::app::{
    App, PreviewDetailCache, ThreadPreviewCacheEntry, THREAD_PREVIEW_CACHE_MAX_ENTRIES,
};
use crate::model::{
    AgentPanel, AgentState, AgentStateSource, AgentType, PreviewSource, PreviewTurn, PreviewView,
    SessionCacheState,
};
use crate::preview_source::PreviewUpdate;
use crate::sidebar::ThreadActivityOverride;
use ratatui::text::Line;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn send_preview_update(app: &mut App, update: PreviewUpdate) {
    let (tx, rx) = mpsc::channel(1);
    tx.blocking_send(update).unwrap();
    app.preview.rx = Some(rx);
    app.check_preview_result();
}

mod latest {
    use super::*;
    include!("preview_tests/latest.rs");
}

mod cache_dirty {
    use super::*;
    include!("preview_tests/cache_dirty.rs");
}

mod selection_scroll {
    use super::*;
    include!("preview_tests/selection_scroll.rs");
}

mod tick_cache {
    use super::*;
    include!("preview_tests/tick_cache.rs");
}
