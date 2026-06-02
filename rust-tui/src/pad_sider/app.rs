use super::fs::{read_file_stats, relative_path_label, FileStats};
use super::index_map::{build_index_map, IndexRow};
use super::preview::{FilePreview, FullscreenPreview, RenderedFilePreview};
use super::preview_cache::FilePreviewCache;
use super::search::FileSearch;
use super::tree::{build_tree, TreeRow};
use crate::codex_turn_diff::TurnDiffEntry;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DYNAMIC_REFRESH_SECS: u64 = 2;
const FULL_REFRESH_SECS: u64 = 30;

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

impl App {
    pub fn new(cwd: PathBuf, target_pane: Option<String>) -> Self {
        let mut expanded = HashSet::new();
        expanded.insert(cwd.clone());
        let mut app = Self {
            cwd,
            target_pane,
            tree: Vec::new(),
            expanded,
            selected: 0,
            index_rows: Vec::new(),
            index_selected: 0,
            codex_diffs: Vec::new(),
            codex_diff_selected: 0,
            codex_diff_preview_key: None,
            focus: Focus::Tree,
            nav_mode: NavMode::Tree,
            selected_stats: FileStats::default(),
            selected_label: String::new(),
            file_preview: FilePreview::empty(),
            file_preview_revision: 0,
            rendered_file_preview: None,
            file_preview_cache: FilePreviewCache::default(),
            preview: None,
            search: None,
            show_help: false,
            show_line_numbers: false,
            text_zoom: 0,
            last_index_toggle_key: None,
            last_refresh: Instant::now() - Duration::from_secs(DYNAMIC_REFRESH_SECS),
            last_full_refresh: Instant::now() - Duration::from_secs(FULL_REFRESH_SECS),
            dirty: true,
            should_quit: false,
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        let selected_path = self.selected_path().cloned();
        self.tree = build_tree(&self.cwd, &self.expanded);
        self.index_rows = build_index_map(&self.cwd);
        if self.index_selected >= self.index_rows.len() {
            self.index_selected = self.index_rows.len().saturating_sub(1);
        }
        self.codex_diffs = crate::codex_turn_diff::list_for_cwd(&self.cwd);
        if self.codex_diff_selected >= self.codex_diffs.len() {
            self.codex_diff_selected = self.codex_diffs.len().saturating_sub(1);
        }
        self.restore_selection(selected_path.as_deref());
        self.refresh_selected();
        self.refresh_file_preview();
        self.refresh_preview();
        self.last_refresh = Instant::now();
        self.last_full_refresh = self.last_refresh;
        self.mark_dirty();
    }

    pub fn tick(&mut self) -> bool {
        if self.last_full_refresh.elapsed() >= Duration::from_secs(FULL_REFRESH_SECS) {
            self.refresh();
            return true;
        }
        if self.last_refresh.elapsed() >= Duration::from_secs(DYNAMIC_REFRESH_SECS) {
            return self.refresh_dynamic_content();
        }
        false
    }

    fn refresh_dynamic_content(&mut self) -> bool {
        let previous_selected_stats = self.selected_stats.clone();
        let previous_selected_label = self.selected_label.clone();
        let previous_file_preview_revision = self.file_preview_revision;
        let previous_preview = self.preview.clone();
        let previous_codex_diffs = self.codex_diffs.clone();

        self.codex_diffs = crate::codex_turn_diff::list_for_cwd(&self.cwd);
        if self.codex_diff_selected >= self.codex_diffs.len() {
            self.codex_diff_selected = self.codex_diffs.len().saturating_sub(1);
        }
        self.refresh_selected();
        self.refresh_file_preview();
        self.refresh_preview();
        self.last_refresh = Instant::now();

        let changed = previous_selected_stats != self.selected_stats
            || previous_selected_label != self.selected_label
            || previous_file_preview_revision != self.file_preview_revision
            || previous_preview != self.preview
            || previous_codex_diffs != self.codex_diffs;

        if changed {
            self.mark_dirty();
        }
        changed
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn take_dirty(&mut self) -> bool {
        let dirty = self.dirty;
        self.dirty = false;
        dirty
    }

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
        let Some(path) = self.selected_path().cloned() else {
            self.selected_label = ".".into();
            self.selected_stats = FileStats::default();
            return;
        };
        self.selected_label = relative_path_label(&self.cwd, &path);
        self.selected_stats = if path.is_file() {
            read_file_stats(&path)
        } else {
            FileStats::default()
        };
    }

    pub(crate) fn refresh_file_preview(&mut self) {
        let path = match self.nav_mode {
            NavMode::Tree => self.selected_path().cloned(),
            NavMode::IndexMap => self.selected_index_path().cloned(),
            NavMode::CodexRuns => {
                self.refresh_codex_diff_preview();
                return;
            }
        };
        self.codex_diff_preview_key = None;
        let previous_title = self.file_preview.title.clone();
        let previous_scroll = self.file_preview.scroll;
        let mut preview = path
            .as_deref()
            .map(|path| self.file_preview_cache.preview_for(&self.cwd, path))
            .unwrap_or_else(FilePreview::empty);
        if preview.title == previous_title {
            preview.scroll = previous_scroll;
        }
        self.set_file_preview(preview);
    }

    pub(crate) fn refresh_preview(&mut self) {
        let Some(preview) = self.preview.as_mut() else {
            return;
        };
        if preview.path.is_file() {
            let scroll = preview.preview.scroll;
            preview.preview = self
                .file_preview_cache
                .preview_for(&self.cwd, &preview.path);
            preview.preview.scroll = scroll;
        } else {
            self.preview = None;
        }
    }
}
