mod lifecycle;
mod preview_state {
    use super::{App, NavMode};
    use crate::pad_sider::preview::FilePreview;

    impl App {
        pub(crate) fn refresh_file_preview(&mut self) {
            let path = match self.nav_mode {
                NavMode::Tree => self.tree.get(self.selected).map(|row| &row.path),
                NavMode::IndexMap => self
                    .index_rows
                    .get(self.index_selected)
                    .map(|row| &row.path),
                NavMode::CodexRuns => {
                    self.refresh_codex_diff_preview();
                    return;
                }
            };
            self.codex_diff_preview_key = None;
            let previous_scroll = self.file_preview.scroll;
            let mut preview = path
                .map(|path| self.file_preview_cache.preview_for(&self.cwd, path))
                .unwrap_or_else(FilePreview::empty);
            if preview.title == self.file_preview.title {
                preview.scroll = previous_scroll;
            }
            self.set_file_preview(preview);
        }

        pub(crate) fn refresh_preview(&mut self) -> bool {
            let Some(preview) = self.preview.as_mut() else {
                return false;
            };
            if preview.path.is_file() {
                let scroll = preview.preview.scroll;
                let mut refreshed = self
                    .file_preview_cache
                    .preview_for(&self.cwd, &preview.path);
                refreshed.scroll = scroll;
                if preview.preview == refreshed {
                    return false;
                }
                preview.preview = refreshed;
                true
            } else {
                self.preview = None;
                true
            }
        }
    }
}
mod selection {
    use super::App;
    use crate::pad_sider::fs::{read_file_stats, relative_path_label, FileStats};
    use std::path::{Path, PathBuf};

    impl App {
        pub fn selected_path(&self) -> Option<&PathBuf> {
            self.tree.get(self.selected).map(|row| &row.path)
        }

        pub fn selected_index_path(&self) -> Option<&PathBuf> {
            self.index_rows
                .get(self.index_selected)
                .map(|row| &row.path)
        }

        pub fn selected_is_dir(&self) -> bool {
            self.tree
                .get(self.selected)
                .map(|row| row.is_dir)
                .unwrap_or(false)
        }

        pub(crate) fn restore_selection(&mut self, selected_path: Option<&Path>) {
            if let Some(path) = selected_path {
                if self.set_selected_path(path) {
                    return;
                }
            }
            if self.selected >= self.tree.len() {
                self.selected = self.tree.len().saturating_sub(1);
            }
        }

        pub(crate) fn set_selected_path(&mut self, path: &Path) -> bool {
            if let Some(index) = self.tree.iter().position(|row| row.path == path) {
                self.selected = index;
                return true;
            }
            false
        }

        pub(crate) fn refresh_selected(&mut self) {
            let Some(path) = self.selected_path() else {
                self.selected_label = ".".into();
                self.selected_stats = FileStats::default();
                return;
            };
            let selected_label = relative_path_label(&self.cwd, path);
            let selected_stats = if path.is_file() {
                read_file_stats(path)
            } else {
                FileStats::default()
            };
            self.selected_label = selected_label;
            self.selected_stats = selected_stats;
        }
    }
}

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
