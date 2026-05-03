use super::fs::{
    is_markdown_file, read_changed_files, read_file_stats, read_markdown_file, relative_path_label,
    FileStats,
};
use super::index_map::{build_index_map, IndexRow};
use super::layout::LayoutWeights;
use super::preview::{FilePreview, MarkdownPreview};
use super::search::FileSearch;
use super::tree::{build_tree, TreeRow};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Tree,
    IndexMap,
    Changes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavMode {
    Tree,
    IndexMap,
}

pub struct App {
    pub cwd: PathBuf,
    pub target_pane: Option<String>,
    pub tree: Vec<TreeRow>,
    pub expanded: HashSet<PathBuf>,
    pub selected: usize,
    pub index_rows: Vec<IndexRow>,
    pub index_selected: usize,
    pub focus: Focus,
    pub nav_mode: NavMode,
    pub layout_weights: LayoutWeights,
    pub changes: Vec<String>,
    pub changes_scroll: u16,
    pub selected_stats: FileStats,
    pub selected_label: String,
    pub file_preview: FilePreview,
    pub preview: Option<MarkdownPreview>,
    pub search: Option<FileSearch>,
    pub show_help: bool,
    pub last_index_toggle_key: Option<Instant>,
    pub last_refresh: Instant,
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
            focus: Focus::Tree,
            nav_mode: NavMode::Tree,
            layout_weights: LayoutWeights::default(),
            changes: Vec::new(),
            changes_scroll: 0,
            selected_stats: FileStats::default(),
            selected_label: String::new(),
            file_preview: FilePreview::empty(),
            preview: None,
            search: None,
            show_help: false,
            last_index_toggle_key: None,
            last_refresh: Instant::now() - Duration::from_secs(5),
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
        self.restore_selection(selected_path.as_deref());
        self.changes = read_changed_files(&self.cwd);
        self.refresh_selected();
        self.refresh_file_preview();
        self.refresh_preview();
        self.last_refresh = Instant::now();
    }

    pub fn tick(&mut self) {
        if self.last_refresh.elapsed() >= Duration::from_secs(2) {
            self.refresh();
        }
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

    pub fn selected_is_markdown(&self) -> bool {
        self.selected_path()
            .map(|path| is_markdown_file(path))
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
        };
        let previous_title = self.file_preview.title.clone();
        let previous_scroll = self.file_preview.scroll;
        let mut preview = path
            .as_deref()
            .map(|path| FilePreview::from_path(&self.cwd, path))
            .unwrap_or_else(FilePreview::empty);
        if preview.title == previous_title {
            preview.scroll = previous_scroll;
        }
        self.file_preview = preview;
    }

    pub(crate) fn refresh_preview(&mut self) {
        let Some(preview) = self.preview.as_mut() else {
            return;
        };
        if preview.path.is_file() {
            preview.content = read_markdown_file(&preview.path);
        } else {
            self.preview = None;
        }
    }
}
