use crate::model::SharedPreviewTurns;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewDetailRenderRequest {
    pub target_key: String,
    pub turns: SharedPreviewTurns,
    pub turn_index: usize,
    pub width: u16,
    pub theme_name: String,
    pub question: String,
    pub answer: Option<String>,
}

#[derive(Clone)]
pub struct PreviewDetailCache {
    pub target_key: String,
    pub turns: SharedPreviewTurns,
    pub turn_index: usize,
    pub width: u16,
    pub theme_name: String,
    pub question: String,
    pub answer: Option<String>,
    pub lines: Vec<ratatui::text::Line<'static>>,
}

#[derive(Clone)]
pub struct PreviewPlainCache {
    pub target_key: String,
    pub width: u16,
    pub theme_name: String,
    pub content_revision: u64,
    pub lines: Vec<ratatui::text::Line<'static>>,
    pub wrapped_rows: usize,
}

#[derive(Clone)]
pub struct PreviewSessionListItemCache {
    pub normal_lines: Vec<ratatui::text::Line<'static>>,
    pub selected_lines: Vec<ratatui::text::Line<'static>>,
}

#[derive(Clone)]
pub struct PreviewSessionListCache {
    pub target_key: String,
    pub width: u16,
    pub theme_name: String,
    pub turns: SharedPreviewTurns,
    pub items: Vec<PreviewSessionListItemCache>,
}

#[derive(Clone)]
pub struct ThreadPreviewCacheEntry {
    pub turns: SharedPreviewTurns,
    pub session_cache_state: Option<crate::model::SessionCacheState>,
    pub transcript_path: Option<String>,
    pub session_id: Option<String>,
    pub updated_at: Option<i64>,
    pub cached_at: i64,
}

#[derive(Clone)]
pub struct PreviewMouseSelection {
    pub anchor_column: u16,
    pub anchor_row: u16,
    pub current_column: u16,
    pub current_row: u16,
}

#[derive(Clone)]
pub struct CopyToast {
    pub title: String,
    pub content_preview: String,
    pub expires_at: std::time::Instant,
}
