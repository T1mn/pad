mod lifecycle;
mod preview_state;
mod selection;

use super::fs::FileStats;
use super::index_map::IndexRow;
use super::preview::{FilePreview, FullscreenPreview, RenderedFilePreview};
use super::preview_cache::FilePreviewCache;
use super::search::FileSearch;
use super::tree::TreeRow;
use crate::codex_turn_diff::TurnDiffEntry;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

pub(super) const DYNAMIC_REFRESH_SECS: u64 = 2;
pub(super) const FULL_REFRESH_SECS: u64 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Tree,
    IndexMap,
    CodexRuns,
    Preview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavMode {
    Tree,
    IndexMap,
    CodexRuns,
}

pub struct App {
    pub cwd: PathBuf,
    pub target_pane: Option<String>,
    pub tree: Vec<TreeRow>,
    pub expanded: HashSet<PathBuf>,
    pub selected: usize,
    pub index_rows: Vec<IndexRow>,
    pub index_selected: usize,
    pub codex_diffs: Vec<TurnDiffEntry>,
    pub codex_diff_selected: usize,
    pub codex_diff_preview_key: Option<String>,
    pub focus: Focus,
    pub nav_mode: NavMode,
    pub selected_stats: FileStats,
    pub selected_label: String,
    pub file_preview: FilePreview,
    pub file_preview_revision: u64,
    pub rendered_file_preview: Option<RenderedFilePreview>,
    pub file_preview_cache: FilePreviewCache,
    pub preview: Option<FullscreenPreview>,
    pub search: Option<FileSearch>,
    pub show_help: bool,
    pub show_line_numbers: bool,
    pub text_zoom: i8,
    pub last_index_toggle_key: Option<Instant>,
    pub last_refresh: Instant,
    pub last_full_refresh: Instant,
    pub dirty: bool,
    pub should_quit: bool,
}
