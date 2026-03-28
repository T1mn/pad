pub mod actions;
pub mod async_ops;
pub mod clipboard;
pub mod hooks;
pub mod navigation;
pub mod preview;
pub mod state;

use crate::fuzzy::FuzzyPicker;
use crate::hook::HookEvent;
use crate::model::{
    AgentPanel, PreviewSessionOrigin, PreviewSource, PreviewTurn, PreviewView, SessionCacheState,
};
use crate::sidebar::{SidebarFolder, SidebarItem, SidebarThread, ThreadActivityOverride};
use crate::theme::{Config, Theme};
use crate::tree;
use async_ops::ScanResult;
use ratatui::text::Line;
use ratatui::widgets::TableState;
use state::{FocusTarget, Mode, RelayView};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

const THREAD_PREVIEW_CACHE_MAX_ENTRIES: usize = 256;
const APP_THREAD_ACTIVITY_MAX_ENTRIES: usize = 256;
const APP_THREAD_ACTIVITY_TTL_SECS: i64 = 12 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewDetailRenderRequest {
    pub target_key: String,
    pub turn_index: usize,
    pub width: u16,
    pub theme_name: String,
    pub question: String,
    pub answer: Option<String>,
}

/// Application state
#[derive(Clone)]
pub struct PreviewDetailCache {
    pub target_key: String,
    pub turn_index: usize,
    pub width: u16,
    pub theme_name: String,
    pub question: String,
    pub answer: Option<String>,
    pub lines: Vec<Line<'static>>,
}

#[derive(Clone)]
pub struct PreviewPlainCache {
    pub target_key: String,
    pub width: u16,
    pub theme_name: String,
    pub content: String,
    pub lines: Vec<Line<'static>>,
    pub wrapped_rows: usize,
}

#[derive(Clone)]
pub struct ThreadPreviewCacheEntry {
    pub turns: Vec<PreviewTurn>,
    pub session_cache_state: Option<SessionCacheState>,
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
    pub expires_at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadActionKind {
    Archive,
    Unarchive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadMetaEditKind {
    Title,
    Tags,
}

#[derive(Clone)]
pub struct PendingThreadAction {
    pub thread: SidebarThread,
    pub kind: ThreadActionKind,
}

pub struct App {
    pub panels: Vec<AgentPanel>,
    pub table_state: TableState,
    pub mode: Mode,
    pub last_refresh: Instant,
    pub search_query: String,
    pub is_searching: bool,
    pub preview_content: String,
    pub preview_pane_id: Option<String>,
    pub preview_source: PreviewSource,
    pub preview_view: PreviewView,
    pub preview_session_origin: Option<PreviewSessionOrigin>,
    pub preview_session_id: Option<String>,
    pub preview_turns: Vec<PreviewTurn>,
    pub preview_selected_turn: Option<usize>,
    pub preview_expanded_turn: Option<usize>,
    pub preview_detail_cache: Option<PreviewDetailCache>,
    pub preview_detail_lru: Vec<PreviewDetailCache>,
    pub preview_detail_render_in_progress: bool,
    pub preview_detail_render_rx: Option<mpsc::Receiver<PreviewDetailCache>>,
    pub preview_detail_pending_request: Option<PreviewDetailRenderRequest>,
    pub preview_plain_cache: Option<PreviewPlainCache>,
    pub thread_preview_cache: HashMap<String, ThreadPreviewCacheEntry>,
    #[allow(dead_code)]
    pub content_hashes: HashMap<String, String>,
    pub settings_open: bool,
    pub config: Config,
    pub locale: crate::i18n::Locale,
    pub theme: Theme,
    pub theme_selector_open: bool,
    pub settings_selected: usize,
    pub theme_selected: usize,
    pub language_selected: usize,
    pub scan_in_progress: bool,
    pub scan_rx: Option<mpsc::Receiver<ScanResult>>,
    pub preview_update_in_progress: bool,
    pub preview_rx: Option<mpsc::Receiver<crate::preview_source::PreviewUpdate>>,
    pub last_preview_update: Instant,
    pub preview_priority_refresh: bool,
    pub hook_rx: Option<mpsc::Receiver<HookEvent>>,
    pub refresh_after_attach: bool,
    pub should_quit: bool,
    pub dirty: bool,
    pub show_tree: bool,
    pub file_tree: Option<tree::FileTree>,
    pub agent_launcher: Option<tree::AgentLauncher>,
    pub delete_target: Option<AgentPanel>,
    pub pending_thread_action: Option<PendingThreadAction>,
    pub thread_meta_editing: bool,
    pub thread_meta_edit_kind: ThreadMetaEditKind,
    pub thread_meta_target: Option<SidebarThread>,
    pub thread_meta_buffer: String,
    pub theme_before_preview: Option<String>,
    pub file_preview_content: String,
    pub file_preview_path: Option<PathBuf>,
    pub file_preview_scroll: u16,
    pub preview_focus: FocusTarget,
    pub preview_scroll: u16,
    pub preview_list_scroll: u16,
    pub preview_detail_scroll: u16,
    pub preview_follow_bottom: bool,
    pub preview_follow_selection: bool,
    pub last_panel_tab_at: Option<Instant>,
    pub expanded_folders: HashSet<String>,
    pub hovered_folder_key: Option<String>,
    pub selected_sidebar_key: Option<String>,
    pub pending_sidebar_selection_index: Option<usize>,
    pub archived_threads_view: bool,
    pub display_session_scope: String,
    pub app_thread_activity: HashMap<String, ThreadActivityOverride>,
    pub sidebar_folders_cache: Vec<SidebarFolder>,
    pub visible_sidebar_items_cache: Vec<SidebarItem>,
    pub sidebar_folders_dirty: bool,
    pub visible_sidebar_items_dirty: bool,
    pub same_session_attached: bool,
    pub pending_status_restore: bool,
    pub saved_tmux_bindings: Vec<String>,
    pub saved_tmux_status: Option<String>,
    pub saved_tmux_status_target: Option<String>,
    pub fuzzy_picker: Option<FuzzyPicker>,
    /// Whether the fuzzy picker was opened from Normal mode (for 'c' key flow)
    pub fuzzy_from_normal: bool,
    // Relay settings state
    pub relay_selected_agent: usize,
    pub relay_selected_provider: usize,
    pub relay_editing: bool,
    pub relay_edit_field: usize, // 0=label, 1=base_url, 2=api_key
    pub relay_edit_buffer: String,
    pub relay_view: RelayView,
    pub settings_search: String,
    pub settings_searching: bool,
    /// Scheduled delayed scan — Some(Instant) means scan after this time
    pub delayed_scan_at: Option<Instant>,
    /// Whether terminal needs a full clear before next draw
    pub needs_clear: bool,
    // Provider connectivity test
    pub provider_test_in_progress: bool,
    pub provider_test_rx:
        Option<mpsc::Receiver<(usize, usize, bool, Option<u16>, Option<u64>, String)>>,
    // Agent style settings
    pub agent_style_selected: usize,
    // Telegram settings
    pub telegram_selected_field: usize,
    pub telegram_editing: bool,
    pub telegram_edit_buffer: String,
    pub busy_animation_frame: usize,
    pub last_busy_animation_tick: Instant,
    pub last_draw_elapsed: Duration,
    pub frame_budget_exceeded: bool,
    pub deferred_hook_events: Vec<HookEvent>,
    pub deferred_scan_result: Option<Vec<AgentPanel>>,
    pub deferred_preview_update: Option<crate::preview_source::PreviewUpdate>,
    pub preview_mouse_selection: Option<PreviewMouseSelection>,
    pub copy_toast: Option<CopyToast>,
}

impl App {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let config = Config::load();
        let display_session_scope = config.display.session_scope.clone();
        let locale = crate::i18n::Locale::from_str(&config.language);
        let theme = Theme::by_name(&config.theme);

        Self {
            panels: Vec::new(),
            table_state,
            mode: Mode::Normal,
            last_refresh: Instant::now(),
            search_query: String::new(),
            is_searching: false,
            preview_content: String::from("Select a panel to preview"),
            preview_pane_id: None,
            preview_source: PreviewSource::Tmux,
            preview_view: PreviewView::Plain,
            preview_session_origin: None,
            preview_session_id: None,
            preview_turns: Vec::new(),
            preview_selected_turn: None,
            preview_expanded_turn: None,
            preview_detail_cache: None,
            preview_detail_lru: Vec::new(),
            preview_detail_render_in_progress: false,
            preview_detail_render_rx: None,
            preview_detail_pending_request: None,
            preview_plain_cache: None,
            thread_preview_cache: HashMap::new(),
            content_hashes: HashMap::new(),
            settings_open: false,
            config,
            locale,
            theme,
            theme_selector_open: false,
            settings_selected: 0,
            theme_selected: 0,
            language_selected: 0,
            scan_in_progress: false,
            scan_rx: None,
            preview_update_in_progress: false,
            preview_rx: None,
            last_preview_update: Instant::now(),
            preview_priority_refresh: false,
            hook_rx: None,
            refresh_after_attach: false,
            should_quit: false,
            dirty: true,
            show_tree: false,
            file_tree: None,
            agent_launcher: None,
            delete_target: None,
            pending_thread_action: None,
            thread_meta_editing: false,
            thread_meta_edit_kind: ThreadMetaEditKind::Title,
            thread_meta_target: None,
            thread_meta_buffer: String::new(),
            theme_before_preview: None,
            file_preview_content: String::new(),
            file_preview_path: None,
            file_preview_scroll: 0,
            preview_focus: FocusTarget::Panel,
            preview_scroll: 0,
            preview_list_scroll: 0,
            preview_detail_scroll: 0,
            preview_follow_bottom: true,
            preview_follow_selection: true,
            last_panel_tab_at: None,
            expanded_folders: HashSet::new(),
            hovered_folder_key: None,
            selected_sidebar_key: None,
            pending_sidebar_selection_index: None,
            archived_threads_view: false,
            display_session_scope,
            app_thread_activity: HashMap::new(),
            sidebar_folders_cache: Vec::new(),
            visible_sidebar_items_cache: Vec::new(),
            sidebar_folders_dirty: true,
            visible_sidebar_items_dirty: true,
            same_session_attached: false,
            pending_status_restore: false,
            saved_tmux_bindings: Vec::new(),
            saved_tmux_status: None,
            saved_tmux_status_target: None,
            fuzzy_picker: None,
            fuzzy_from_normal: false,
            relay_selected_agent: 0,
            relay_selected_provider: 0,
            relay_editing: false,
            relay_edit_field: 0,
            relay_edit_buffer: String::new(),
            relay_view: RelayView::AgentList,
            settings_search: String::new(),
            settings_searching: false,
            delayed_scan_at: None,
            needs_clear: false,
            provider_test_in_progress: false,
            provider_test_rx: None,
            agent_style_selected: 0,
            telegram_selected_field: 0,
            telegram_editing: false,
            telegram_edit_buffer: String::new(),
            busy_animation_frame: 0,
            last_busy_animation_tick: Instant::now(),
            last_draw_elapsed: Duration::default(),
            frame_budget_exceeded: false,
            deferred_hook_events: Vec::new(),
            deferred_scan_result: None,
            deferred_preview_update: None,
            preview_mouse_selection: None,
            copy_toast: None,
        }
    }

    pub fn apply_theme(&mut self, name: &str) {
        self.config.theme = name.to_string();
        self.theme = Theme::by_name(name);
        self.config.save();
        self.theme_before_preview = None;
        self.clear_preview_render_caches();
        self.dirty = true;
    }

    pub fn preview_theme(&mut self, name: &str) {
        self.theme = Theme::by_name(name);
        self.clear_preview_render_caches();
        self.dirty = true;
    }

    pub fn invalidate_sidebar_cache(&mut self) {
        self.sidebar_folders_dirty = true;
        self.visible_sidebar_items_dirty = true;
    }

    pub fn invalidate_sidebar_visible_cache(&mut self) {
        self.visible_sidebar_items_dirty = true;
    }

    pub fn showing_live_sessions(&self) -> bool {
        self.display_session_scope == "live"
    }

    pub fn apply_display_session_scope(&mut self, scope: &str, persist_default: bool) -> bool {
        let normalized = if scope == "all" { "all" } else { "live" };
        let runtime_changed = self.display_session_scope != normalized;
        let config_changed = self.config.display.session_scope != normalized;

        if persist_default && config_changed {
            self.config.display.session_scope = normalized.to_string();
            self.config.save();
        }

        if runtime_changed {
            self.display_session_scope = normalized.to_string();
            self.pending_thread_action = None;
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
            self.invalidate_preview();
            self.focus_panel();
            self.dirty = true;
        } else if persist_default && config_changed {
            self.dirty = true;
        }

        runtime_changed || (persist_default && config_changed)
    }

    pub fn toggle_display_session_scope_view(&mut self) -> bool {
        if self.archived_threads_view {
            return false;
        }
        let next_scope = if self.showing_live_sessions() {
            "all"
        } else {
            "live"
        };
        self.apply_display_session_scope(next_scope, false)
    }
}

pub(crate) fn unix_now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
