use super::FocusTarget;
use crate::model::{PreviewSessionOrigin, PreviewSource, PreviewView, SharedPreviewTurns};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewDetailRenderRequest {
    pub target_key: String,
    pub turn_index: usize,
    pub width: u16,
    pub theme_name: String,
    pub question: String,
    pub answer: Option<String>,
}

#[derive(Clone)]
pub struct PreviewDetailCache {
    pub target_key: String,
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
    pub content: String,
    pub lines: Vec<ratatui::text::Line<'static>>,
    pub wrapped_rows: usize,
}

#[derive(Clone)]
pub struct PreviewSessionListItemCache {
    pub question: String,
    pub answer: Option<String>,
    pub normal_lines: Vec<ratatui::text::Line<'static>>,
    pub selected_lines: Vec<ratatui::text::Line<'static>>,
}

#[derive(Clone)]
pub struct PreviewSessionListCache {
    pub target_key: String,
    pub width: u16,
    pub theme_name: String,
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

pub struct PreviewState {
    pub content: String,
    pub pane_id: Option<String>,
    pub source: PreviewSource,
    pub view: PreviewView,
    pub session_origin: Option<PreviewSessionOrigin>,
    pub session_id: Option<String>,
    pub turns: SharedPreviewTurns,
    pub selected_turn: Option<usize>,
    pub expanded_turn: Option<usize>,
    pub detail_cache: Option<PreviewDetailCache>,
    pub detail_lru: Vec<PreviewDetailCache>,
    pub detail_render_in_progress: bool,
    pub detail_render_rx: Option<mpsc::Receiver<PreviewDetailCache>>,
    pub detail_pending_request: Option<PreviewDetailRenderRequest>,
    pub plain_cache: Option<PreviewPlainCache>,
    pub session_list_cache: Option<PreviewSessionListCache>,
    pub thread_preview_cache: HashMap<String, ThreadPreviewCacheEntry>,
    pub last_preview_update: Instant,
    pub priority_refresh: bool,
    pub update_in_progress: bool,
    pub rx: Option<mpsc::Receiver<crate::preview_source::PreviewUpdate>>,
    pub pending_update_request: Option<crate::preview_source::PreviewRequest>,
    pub navigation_debounce_until: Option<Instant>,
    pub theme_before_preview: Option<String>,
    pub focus: FocusTarget,
    pub scroll: u16,
    pub list_scroll: u16,
    pub detail_scroll: u16,
    pub follow_bottom: bool,
    pub follow_selection: bool,
    pub last_panel_tab_at: Option<Instant>,
    pub last_detail_exit_tab_at: Option<Instant>,
    pub file_preview_content: String,
    pub file_preview_path: Option<PathBuf>,
    pub file_preview_scroll: u16,
    pub mouse_selection: Option<PreviewMouseSelection>,
    pub copy_toast: Option<CopyToast>,
    pub deferred_preview_update: Option<crate::preview_source::PreviewUpdate>,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            content: String::from("Select a panel to preview"),
            pane_id: None,
            source: PreviewSource::Tmux,
            view: PreviewView::Plain,
            session_origin: None,
            session_id: None,
            turns: SharedPreviewTurns::default(),
            selected_turn: None,
            expanded_turn: None,
            detail_cache: None,
            detail_lru: Vec::new(),
            detail_render_in_progress: false,
            detail_render_rx: None,
            detail_pending_request: None,
            plain_cache: None,
            session_list_cache: None,
            thread_preview_cache: HashMap::new(),
            last_preview_update: Instant::now(),
            priority_refresh: false,
            update_in_progress: false,
            rx: None,
            pending_update_request: None,
            navigation_debounce_until: None,
            theme_before_preview: None,
            focus: FocusTarget::Panel,
            scroll: 0,
            list_scroll: 0,
            detail_scroll: 0,
            follow_bottom: true,
            follow_selection: true,
            last_panel_tab_at: None,
            last_detail_exit_tab_at: None,
            file_preview_content: String::new(),
            file_preview_path: None,
            file_preview_scroll: 0,
            mouse_selection: None,
            copy_toast: None,
            deferred_preview_update: None,
        }
    }

    pub fn is_focused(&self) -> bool {
        self.focus == FocusTarget::Preview
    }

    pub fn uses_list_scroll(&self) -> bool {
        self.source == PreviewSource::Session && self.view == PreviewView::SessionList
    }

    pub fn uses_detail_scroll(&self) -> bool {
        self.source == PreviewSource::Session && self.view == PreviewView::SessionDetail
    }

    pub fn mouse_selection(&self) -> Option<&PreviewMouseSelection> {
        self.mouse_selection.as_ref()
    }
}

impl Default for PreviewState {
    fn default() -> Self {
        Self::new()
    }
}
