use super::{App, Focus, NavMode, DYNAMIC_REFRESH_SECS, FULL_REFRESH_SECS};
use crate::codex_turn_diff;
use crate::pad_sider::fs::FileStats;
use crate::pad_sider::index_map::build_index_map;
use crate::pad_sider::preview::FilePreview;
use crate::pad_sider::preview_cache::FilePreviewCache;
use crate::pad_sider::tree::build_tree;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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
        self.codex_diffs = codex_turn_diff::list_for_cwd(&self.cwd);
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

        self.codex_diffs = codex_turn_diff::list_for_cwd(&self.cwd);
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
}
