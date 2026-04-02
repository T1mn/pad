use super::App;
use crate::app::state::sidebar::{PendingSidebarSpaceAction, PendingSidebarSpaceActionKind};
use crate::log_debug;
use crate::model::AgentPanel;
use crate::sidebar::{SidebarFolder, SidebarItem, SidebarThread};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

impl App {
    fn visible_thread_sidebar_indices(items: &[SidebarItem]) -> Vec<usize> {
        items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.as_thread().map(|_| index))
            .collect()
    }

    fn sidebar_navigable_indices_for(items: &[SidebarItem]) -> Vec<usize> {
        items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                if Self::sidebar_item_is_navigable(items, index, item) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect()
    }

    fn sidebar_item_is_navigable(items: &[SidebarItem], index: usize, item: &SidebarItem) -> bool {
        match item {
            SidebarItem::Thread(_) => true,
            SidebarItem::Folder(folder) => items
                .get(index + 1)
                .and_then(SidebarItem::as_thread)
                .is_none_or(|thread| thread.folder_key != folder.key),
        }
    }

    fn next_navigable_sidebar_index(
        items: &[SidebarItem],
        current: Option<usize>,
        forward: bool,
    ) -> Option<usize> {
        let candidates = Self::sidebar_navigable_indices_for(items);
        if candidates.is_empty() {
            return None;
        }

        match current {
            Some(current) if forward => candidates
                .iter()
                .copied()
                .find(|index| *index > current)
                .or_else(|| candidates.first().copied()),
            Some(current) => candidates
                .iter()
                .rev()
                .copied()
                .find(|index| *index < current)
                .or_else(|| candidates.last().copied()),
            None if forward => candidates.first().copied(),
            None => candidates.last().copied(),
        }
    }

    fn apply_cached_preview_to_thread(&self, thread: &mut SidebarThread) {
        let Some(cache) = self.preview.thread_preview_cache.get(&thread.key) else {
            return;
        };

        if cache.turns.len() > thread.cached_preview_turns.len() {
            thread.cached_preview_turns = cache.turns.clone();
        }
        if thread.session_cache_state.is_none() {
            thread.session_cache_state = cache.session_cache_state;
        }
        if thread.transcript_path.is_none() {
            thread.transcript_path = cache.transcript_path.clone();
        }
        if thread.session_id.is_none() {
            thread.session_id = cache.session_id.clone();
        }
        if let Some(updated_at) = cache.updated_at {
            thread.updated_at = thread.updated_at.max(updated_at);
        }
        if (thread.title.trim().is_empty() || thread.title == "untitled") && !cache.turns.is_empty()
        {
            thread.title = cache.turns[0]
                .question
                .lines()
                .next()
                .unwrap_or("untitled")
                .trim()
                .to_string();
        }
    }

    fn ensure_sidebar_folders_cache(&mut self) {
        if self.sidebar.sidebar_folders_dirty {
            let started_at = std::time::Instant::now();
            let overrides = if self.sidebar.archived_threads_view {
                Vec::new()
            } else {
                self.prune_app_thread_activity(crate::app::unix_now_ts());
                self.sidebar
                    .app_thread_activity
                    .values()
                    .cloned()
                    .collect::<Vec<_>>()
            };
            let empty_startup_thread_sort_activity = HashMap::new();
            let startup_thread_sort_activity = if self.sidebar.archived_threads_view {
                &empty_startup_thread_sort_activity
            } else {
                &self.sidebar.startup_thread_sort_activity
            };
            let mut folders = crate::sidebar::build_sidebar_folders(
                &self.panels,
                &overrides,
                &self.sidebar.thread_sort_activity,
                startup_thread_sort_activity,
                self.sidebar.archived_threads_view,
                !self.sidebar.archived_threads_view && self.showing_live_sessions(),
            );
            for folder in &mut folders {
                for thread in &mut folder.threads {
                    self.apply_cached_preview_to_thread(thread);
                }
                folder.threads.sort_by(crate::sidebar::thread_sort_key);
                folder.updated_at = folder
                    .threads
                    .first()
                    .map(|thread| thread.sort_timestamp())
                    .unwrap_or_default();
            }
            folders.sort_by(crate::sidebar::folder_sort_key);
            self.sidebar.sidebar_folders_cache = folders;
            self.sidebar.sidebar_folders_dirty = false;
            self.sidebar.visible_sidebar_items_dirty = true;
            let elapsed = started_at.elapsed();
            if elapsed >= std::time::Duration::from_millis(8) {
                log_debug!(
                    "sidebar.cache: rebuild_folders folders={} elapsed_ms={}",
                    self.sidebar.sidebar_folders_cache.len(),
                    elapsed.as_millis()
                );
            }
        }
    }

    pub fn seed_startup_thread_sort_activity_once(&mut self) -> bool {
        if self.sidebar.startup_thread_sort_seeded {
            return false;
        }

        let folders = crate::sidebar::build_sidebar_folders(
            &self.panels,
            &[],
            &HashMap::new(),
            &HashMap::new(),
            false,
            false,
        );

        let mut seeded_threads = 0usize;
        let mut seeded_keys = 0usize;
        for folder in folders {
            for thread in folder.threads {
                if thread.archived || thread.updated_at <= 0 {
                    continue;
                }
                seeded_threads += 1;
                for key in thread.sort_activity_keys() {
                    let entry = self
                        .sidebar
                        .startup_thread_sort_activity
                        .entry(key)
                        .or_insert(thread.updated_at);
                    if thread.updated_at > *entry {
                        *entry = thread.updated_at;
                    }
                    seeded_keys += 1;
                }
            }
        }

        self.sidebar.startup_thread_sort_seeded = true;
        log_debug!(
            "sidebar.startup_sort: seeded threads={} candidate_keys={} unique_keys={} panels={}",
            seeded_threads,
            seeded_keys,
            self.sidebar.startup_thread_sort_activity.len(),
            self.panels.len()
        );
        true
    }

    fn ensure_visible_sidebar_items_cache(&mut self) {
        if self.sidebar.visible_sidebar_items_dirty {
            let started_at = std::time::Instant::now();
            self.ensure_sidebar_folders_cache();
            self.sidebar.visible_sidebar_items_cache = crate::sidebar::build_visible_sidebar_items(
                &self.sidebar.sidebar_folders_cache,
                &self.sidebar.expanded_folders,
                &self.search_query,
            );
            self.sidebar.visible_sidebar_items_dirty = false;
            let elapsed = started_at.elapsed();
            if elapsed >= std::time::Duration::from_millis(8) {
                log_debug!(
                    "sidebar.cache: rebuild_visible items={} elapsed_ms={}",
                    self.sidebar.visible_sidebar_items_cache.len(),
                    elapsed.as_millis()
                );
            }
        }
    }

    pub fn sidebar_folders_ref(&mut self) -> &[SidebarFolder] {
        self.ensure_sidebar_folders_cache();
        &self.sidebar.sidebar_folders_cache
    }

    pub fn visible_sidebar_items_ref(&mut self) -> &[SidebarItem] {
        self.ensure_visible_sidebar_items_cache();
        &self.sidebar.visible_sidebar_items_cache
    }

    #[allow(dead_code)]
    pub fn sidebar_folders(&mut self) -> Vec<SidebarFolder> {
        self.sidebar_folders_ref().to_vec()
    }

    #[allow(dead_code)]
    pub fn visible_sidebar_items(&mut self) -> Vec<SidebarItem> {
        self.visible_sidebar_items_ref().to_vec()
    }

    pub fn sync_sidebar_selection(&mut self) {
        let folders = self.sidebar_folders_ref().to_vec();
        let items = self.visible_sidebar_items_ref().to_vec();

        if items.is_empty() {
            self.sidebar.selected_sidebar_key = None;
            self.table_state.select(None);
            return;
        }

        let mut selected_key = self.sidebar.selected_sidebar_key.clone();
        let mut selected_index = selected_key
            .as_deref()
            .and_then(|key| items.iter().position(|item| item.key() == key));

        if selected_index.is_none() {
            if let Some(folder_key) = selected_key.as_deref().and_then(|key| {
                folders.iter().find_map(|folder| {
                    if folder.key == key || folder.threads.iter().any(|thread| thread.key == key) {
                        Some(folder.key.clone())
                    } else {
                        None
                    }
                })
            }) {
                selected_index = items.iter().position(|item| item.key() == folder_key);
                if selected_index.is_some() {
                    selected_key = Some(folder_key);
                }
            }
        }

        if selected_index.is_none() {
            if let Some(preferred_index) = self.sidebar.pending_sidebar_selection_index.take() {
                let clamped_index = preferred_index.min(items.len().saturating_sub(1));
                selected_index = Some(clamped_index);
                selected_key = items.get(clamped_index).map(|item| item.key().to_string());
            }
        }

        if selected_index.is_none() {
            selected_index = Some(0);
            selected_key = Some(items[0].key().to_string());
        }

        self.sidebar.selected_sidebar_key = selected_key;
        self.table_state.select(selected_index);
    }

    pub fn selected_sidebar_item(&mut self) -> Option<SidebarItem> {
        let selected_key = self.sidebar.selected_sidebar_key.clone();
        let selected_index = self.table_state.selected();
        let items = self.visible_sidebar_items_ref();
        if items.is_empty() {
            return None;
        }

        if let Some(key) = selected_key.as_deref() {
            if let Some(item) = items.iter().find(|item| item.key() == key) {
                return Some(item.clone());
            }
        }

        selected_index
            .and_then(|index| items.get(index).cloned())
            .or_else(|| items.first().cloned())
    }

    pub fn selected_preview_thread(&mut self) -> Option<SidebarThread> {
        if self.sidebar.selected_sidebar_key.is_none() && !self.sidebar.archived_threads_view {
            if let Some(panel) = self
                .table_state
                .selected()
                .and_then(|index| self.panels.get(index))
            {
                let mut thread = crate::sidebar::thread_from_live_panel(panel);
                self.apply_cached_preview_to_thread(&mut thread);
                return Some(thread);
            }
        }

        let mut thread = match self.selected_sidebar_item()? {
            SidebarItem::Folder(folder) => folder.primary_thread()?,
            SidebarItem::Thread(thread) => thread,
        };
        self.apply_cached_preview_to_thread(&mut thread);
        Some(thread)
    }

    pub fn select_sidebar_index(&mut self, index: usize, invalidate_preview: bool) -> bool {
        let visible_len = self.visible_sidebar_items_ref().len();
        if index >= visible_len {
            return false;
        }

        let selected_key = self
            .visible_sidebar_items_ref()
            .get(index)
            .map(|item| item.key().to_string());
        self.table_state.select(Some(index));
        self.sidebar.selected_sidebar_key = selected_key;
        self.clear_unread_stop_for_selected_panel();
        if invalidate_preview {
            self.invalidate_preview();
        }
        self.update_tree_for_selection();
        self.dirty = true;
        true
    }

    pub fn next(&mut self) {
        if self.sidebar.show_tree {
            if let Some(ref mut tree) = self.sidebar.file_tree {
                log_debug!("nav: next (tree) selected={:?}", tree.state.selected());
                tree.next();
                self.dirty = true;
                return;
            }
        }
        let visible_len = self.visible_sidebar_items_ref().len();
        if visible_len == 0 {
            self.table_state.select(None);
            self.sidebar.selected_sidebar_key = None;
            return;
        }

        let items = self.visible_sidebar_items_ref().to_vec();
        let current = self.table_state.selected();
        let Some(i) = Self::next_navigable_sidebar_index(&items, current, true) else {
            return;
        };
        let selected_key = self
            .visible_sidebar_items_ref()
            .get(i)
            .map(|item| item.key().to_string());
        log_debug!("nav: next (panel) index={}", i);
        self.table_state.select(Some(i));
        self.sidebar.selected_sidebar_key = selected_key;
        self.clear_unread_stop_for_selected_panel();
        self.invalidate_preview();
        self.update_tree_for_selection();
        self.dirty = true;
    }

    pub fn previous(&mut self) {
        if self.sidebar.show_tree {
            if let Some(ref mut tree) = self.sidebar.file_tree {
                log_debug!("nav: previous (tree) selected={:?}", tree.state.selected());
                tree.previous();
                self.dirty = true;
                return;
            }
        }
        let visible_len = self.visible_sidebar_items_ref().len();
        if visible_len == 0 {
            self.table_state.select(None);
            self.sidebar.selected_sidebar_key = None;
            return;
        }

        let items = self.visible_sidebar_items_ref().to_vec();
        let current = self.table_state.selected();
        let Some(i) = Self::next_navigable_sidebar_index(&items, current, false) else {
            return;
        };
        let selected_key = self
            .visible_sidebar_items_ref()
            .get(i)
            .map(|item| item.key().to_string());
        log_debug!("nav: previous (panel) index={}", i);
        self.table_state.select(Some(i));
        self.sidebar.selected_sidebar_key = selected_key;
        self.clear_unread_stop_for_selected_panel();
        self.invalidate_preview();
        self.update_tree_for_selection();
        self.dirty = true;
    }

    pub fn jump_to_sidebar_index(&mut self, index: usize) -> bool {
        self.select_sidebar_index(index, true)
    }

    pub fn jump_to(&mut self, index: usize) {
        let target_sidebar_index =
            Self::visible_thread_sidebar_indices(self.visible_sidebar_items_ref())
                .get(index)
                .copied();
        let Some(target_sidebar_index) = target_sidebar_index else {
            return;
        };
        self.select_sidebar_index(target_sidebar_index, true);
    }

    pub fn toggle_selected_folder(&mut self) -> bool {
        let Some(item) = self.selected_sidebar_item() else {
            return false;
        };
        let Some(folder) = item.as_folder() else {
            return false;
        };
        if self.sidebar.expanded_folders.contains(&folder.key) {
            self.sidebar.expanded_folders.remove(&folder.key);
        } else {
            self.sidebar.expanded_folders.insert(folder.key.clone());
        }
        self.invalidate_sidebar_visible_cache();
        self.sync_sidebar_selection();
        self.invalidate_preview();
        self.dirty = true;
        true
    }

    pub fn queue_pending_sidebar_space_action(&mut self, window: Duration) -> bool {
        if self.sidebar.show_tree || self.preview_is_focused() {
            return false;
        }

        let kind = match self.selected_sidebar_item() {
            Some(SidebarItem::Folder(folder)) => {
                PendingSidebarSpaceActionKind::ToggleFolder(folder.key.clone())
            }
            Some(SidebarItem::Thread(thread)) => {
                PendingSidebarSpaceActionKind::CollapseParentFolder(thread.folder_key.clone())
            }
            None => return false,
        };

        self.sidebar.pending_space_action = Some(PendingSidebarSpaceAction {
            kind,
            deadline: Instant::now() + window,
        });
        true
    }

    pub fn pending_sidebar_space_action_is_active(&self) -> bool {
        self.sidebar
            .pending_space_action
            .as_ref()
            .map(|action| action.deadline > Instant::now())
            .unwrap_or(false)
    }

    pub fn clear_pending_sidebar_space_action(&mut self) {
        self.sidebar.pending_space_action = None;
    }

    pub fn flush_pending_sidebar_space_action_if_due(&mut self) -> bool {
        if self
            .sidebar
            .pending_space_action
            .as_ref()
            .map(|action| action.deadline <= Instant::now())
            .unwrap_or(false)
        {
            return self.flush_pending_sidebar_space_action();
        }

        false
    }

    pub fn flush_pending_sidebar_space_action(&mut self) -> bool {
        let Some(action) = self.sidebar.pending_space_action.take() else {
            return false;
        };

        match action.kind {
            PendingSidebarSpaceActionKind::ToggleFolder(folder_key) => {
                let folder_exists = self
                    .sidebar_folders_ref()
                    .iter()
                    .any(|folder| folder.key == folder_key);
                if !folder_exists {
                    return false;
                }
                if self.sidebar.expanded_folders.contains(&folder_key) {
                    self.sidebar.expanded_folders.remove(&folder_key);
                } else {
                    self.sidebar.expanded_folders.insert(folder_key.clone());
                }
                self.sidebar.selected_sidebar_key = Some(folder_key);
                self.invalidate_sidebar_visible_cache();
                self.sync_sidebar_selection();
                self.invalidate_preview();
                self.dirty = true;
                true
            }
            PendingSidebarSpaceActionKind::CollapseParentFolder(folder_key) => {
                if !self.sidebar.expanded_folders.remove(&folder_key) {
                    return false;
                }
                self.sidebar.selected_sidebar_key = Some(folder_key);
                self.invalidate_sidebar_visible_cache();
                self.sync_sidebar_selection();
                self.focus_panel();
                self.invalidate_preview();
                self.dirty = true;
                true
            }
        }
    }

    pub fn toggle_all_sidebar_folders(&mut self) -> bool {
        let folder_keys = self
            .sidebar_folders_ref()
            .iter()
            .map(|folder| folder.key.clone())
            .collect::<Vec<_>>();
        if folder_keys.is_empty() {
            return false;
        }

        let collapse_all = folder_keys
            .iter()
            .any(|key| self.sidebar.expanded_folders.contains(key));

        if collapse_all {
            for key in &folder_keys {
                self.sidebar.expanded_folders.remove(key);
            }
            if let Some(SidebarItem::Thread(thread)) = self.selected_sidebar_item() {
                self.sidebar.selected_sidebar_key = Some(thread.folder_key.clone());
            }
        } else {
            for key in &folder_keys {
                self.sidebar.expanded_folders.insert(key.clone());
            }
        }

        self.invalidate_sidebar_visible_cache();
        self.sync_sidebar_selection();
        self.focus_panel();
        self.invalidate_preview();
        self.dirty = true;
        true
    }

    pub fn expand_selected_folder(&mut self) -> bool {
        let Some(item) = self.selected_sidebar_item() else {
            return false;
        };
        let Some(folder) = item.as_folder() else {
            return false;
        };
        if self.sidebar.expanded_folders.insert(folder.key.clone()) {
            self.invalidate_sidebar_visible_cache();
            self.sync_sidebar_selection();
            self.invalidate_preview();
            self.dirty = true;
        }
        true
    }

    pub fn collapse_selected_folder(&mut self) -> bool {
        let Some(item) = self.selected_sidebar_item() else {
            return false;
        };
        match item {
            SidebarItem::Folder(folder) => {
                if self.sidebar.expanded_folders.remove(&folder.key) {
                    self.invalidate_sidebar_visible_cache();
                    self.sync_sidebar_selection();
                    self.invalidate_preview();
                    self.dirty = true;
                }
                true
            }
            SidebarItem::Thread(thread) => {
                self.sidebar.selected_sidebar_key = Some(thread.folder_key.clone());
                self.sync_sidebar_selection();
                self.focus_panel();
                self.invalidate_preview();
                self.dirty = true;
                true
            }
        }
    }

    pub fn collapse_parent_folder_for_selected_thread(&mut self) -> bool {
        let Some(SidebarItem::Thread(thread)) = self.selected_sidebar_item() else {
            return false;
        };
        if !self.sidebar.expanded_folders.remove(&thread.folder_key) {
            return false;
        }
        self.sidebar.selected_sidebar_key = Some(thread.folder_key.clone());
        self.invalidate_sidebar_visible_cache();
        self.sync_sidebar_selection();
        self.focus_panel();
        self.invalidate_preview();
        self.dirty = true;
        true
    }

    #[allow(dead_code)]
    pub fn filtered_panels(&self) -> Vec<&AgentPanel> {
        if self.search_query.is_empty() {
            self.panels.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.panels
                .iter()
                .filter(|p| {
                    p.session.to_lowercase().contains(&query)
                        || p.window.to_lowercase().contains(&query)
                        || p.working_dir.to_lowercase().contains(&query)
                })
                .collect()
        }
    }

    pub fn selected_panel(&mut self) -> Option<&AgentPanel> {
        let pane_id = self
            .selected_preview_thread()
            .and_then(|thread| thread.live_pane_id)?;
        self.panels.iter().find(|panel| panel.pane_id == pane_id)
    }

    pub fn update_tree_for_selection(&mut self) {
        if self.sidebar.show_tree {
            if let Some(thread) = self.selected_preview_thread() {
                let path = PathBuf::from(&thread.working_dir);
                if path.exists() {
                    let should_update = match &self.sidebar.file_tree {
                        None => true,
                        Some(tree) => tree.root_path != path,
                    };
                    if should_update {
                        self.sidebar.file_tree = Some(crate::tree::FileTree::new(path));
                        self.dirty = true;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

    fn sample_panel(pane_id: &str, working_dir: &str) -> AgentPanel {
        AgentPanel {
            session: "0".into(),
            window: "main".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: pane_id.into(),
            agent_type: AgentType::Codex,
            working_dir: working_dir.into(),
            is_active: true,
            state: AgentState::Idle,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        }
    }

    #[test]
    fn next_uses_folder_rows_when_not_expanded() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.sync_sidebar_selection();

        app.next();

        assert_eq!(app.table_state.selected(), Some(1));
        assert_eq!(
            app.sidebar.selected_sidebar_key.as_deref(),
            Some("/tmp/beta")
        );
    }

    #[test]
    fn next_skips_expanded_folder_row() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.sidebar.expanded_folders.insert("/tmp/alpha".into());
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();

        app.next();

        assert_eq!(app.table_state.selected(), Some(1));
        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));
    }

    #[test]
    fn next_skips_search_expanded_folder_row() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.search_query = "alpha".into();
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();

        app.next();

        assert_eq!(app.table_state.selected(), Some(1));
        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));
    }

    #[test]
    fn numeric_jump_ignores_folder_rows_and_hidden_threads() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.sidebar.expanded_folders.insert("/tmp/alpha".into());
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();

        app.jump_to(0);
        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));

        app.jump_to(1);
        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));
    }

    #[test]
    fn numeric_jump_uses_visible_thread_order() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.sidebar.expanded_folders.insert("/tmp/alpha".into());
        app.sidebar.expanded_folders.insert("/tmp/beta".into());
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();

        app.jump_to(1);

        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%2"));
        assert_eq!(app.table_state.selected(), Some(3));
    }

    #[test]
    fn numeric_jump_uses_filtered_visible_threads() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.search_query = "beta".into();
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();

        app.jump_to(0);

        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%2"));
    }
}
