pub mod actions;
pub mod async_ops;
pub mod clipboard;
pub mod hooks;
pub mod navigation;
pub mod preview;
pub mod state;

use crate::fuzzy::FuzzyPicker;
use crate::hook::HookEvent;
use crate::model::AgentPanel;
use crate::theme::{Config, Theme};
pub use async_ops::CodexCliVersionInfo;
use async_ops::{CodexCliUpdateResult, CodexCliVersionCheckResult, ProviderTestResult, ScanResult};
use ratatui::widgets::TableState;
use state::{
    CodexSettingsView, Mode, PreviewState, RelayPopupMode, RelayView, SettingsDetailKind,
    SettingsFocus, SidebarState, ThreadListView,
};
pub use state::{
    CopyToast, PendingThreadAction, PreviewDetailCache, PreviewDetailRenderRequest,
    PreviewMouseSelection, PreviewPlainCache, PreviewSessionListCache, PreviewSessionListItemCache,
    ThreadActionKind, ThreadMetaEditKind, ThreadPreviewCacheEntry,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

const THREAD_PREVIEW_CACHE_MAX_ENTRIES: usize = 256;
const APP_THREAD_ACTIVITY_MAX_ENTRIES: usize = 256;
const APP_THREAD_ACTIVITY_TTL_SECS: i64 = 12 * 60 * 60;
pub struct App {
    pub panels: Vec<AgentPanel>,
    pub table_state: TableState,
    pub mode: Mode,
    pub last_refresh: Instant,
    pub search_query: String,
    pub is_searching: bool,
    pub sidebar: SidebarState,
    pub preview: PreviewState,
    pub settings_open: bool,
    pub config: Config,
    pub locale: crate::i18n::Locale,
    pub theme: Theme,
    pub theme_selector_open: bool,
    pub settings_selected: usize,
    pub settings_focus: SettingsFocus,
    pub active_settings_detail: Option<SettingsDetailKind>,
    pub theme_selected: usize,
    pub language_selected: usize,
    pub scan_in_progress: bool,
    pub scan_rx: Option<mpsc::Receiver<ScanResult>>,
    pub hook_rx: Option<mpsc::Receiver<HookEvent>>,
    pub refresh_after_attach: bool,
    pub should_quit: bool,
    pub dirty: bool,
    pub same_session_attached: bool,
    pub same_session_trace_id: Option<String>,
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
    pub relay_popup_mode: RelayPopupMode,
    pub relay_popup_selected: usize,
    pub relay_popup_field: usize,
    pub relay_popup_editing: bool,
    pub relay_popup_buffer: String,
    pub settings_search: String,
    pub settings_searching: bool,
    /// Scheduled delayed scan — Some(Instant) means scan after this time
    pub delayed_scan_at: Option<Instant>,
    /// Whether terminal needs a full clear before next draw
    pub needs_clear: bool,
    // Provider connectivity test
    pub provider_test_in_progress: bool,
    pub provider_test_rx: Option<mpsc::Receiver<ProviderTestResult>>,
    pub codex_cli_check_in_progress: bool,
    pub codex_cli_check_rx: Option<mpsc::Receiver<CodexCliVersionCheckResult>>,
    pub codex_cli_update_in_progress: bool,
    pub codex_cli_update_rx: Option<mpsc::Receiver<CodexCliUpdateResult>>,
    pub codex_cli_version_info: Option<CodexCliVersionInfo>,
    pub title_summary_tx: Option<mpsc::Sender<crate::title_summary::TitleSummaryResult>>,
    pub title_summary_rx: Option<mpsc::Receiver<crate::title_summary::TitleSummaryResult>>,
    pub title_summary_in_flight: HashSet<String>,
    // Agent style settings
    pub agent_style_selected: usize,
    pub codex_settings_view: CodexSettingsView,
    pub codex_settings_category_selected: usize,
    pub codex_settings_selected: usize,
    pub sound_settings_selected: usize,
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
    pub notification_inbox: crate::notification_inbox::NotificationInbox,
    pub notification_inbox_selected: usize,
    relay_config_last_poll_at: Instant,
    relay_config_source_path: Option<PathBuf>,
    relay_config_source_modified_ms: Option<u128>,
    relay_config_source_len: Option<u64>,
    pending_external_relay_reload: bool,
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
            sidebar: SidebarState::new(display_session_scope),
            preview: PreviewState::new(),
            settings_open: false,
            config,
            locale,
            theme,
            theme_selector_open: false,
            settings_selected: 0,
            settings_focus: SettingsFocus::List,
            active_settings_detail: None,
            theme_selected: 0,
            language_selected: 0,
            scan_in_progress: false,
            scan_rx: None,
            hook_rx: None,
            refresh_after_attach: false,
            should_quit: false,
            dirty: true,
            same_session_attached: false,
            same_session_trace_id: None,
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
            relay_popup_mode: RelayPopupMode::None,
            relay_popup_selected: 0,
            relay_popup_field: 0,
            relay_popup_editing: false,
            relay_popup_buffer: String::new(),
            settings_search: String::new(),
            settings_searching: false,
            delayed_scan_at: None,
            needs_clear: false,
            provider_test_in_progress: false,
            provider_test_rx: None,
            codex_cli_check_in_progress: false,
            codex_cli_check_rx: None,
            codex_cli_update_in_progress: false,
            codex_cli_update_rx: None,
            codex_cli_version_info: None,
            title_summary_tx: None,
            title_summary_rx: None,
            title_summary_in_flight: HashSet::new(),
            agent_style_selected: 0,
            codex_settings_view: CodexSettingsView::Categories,
            codex_settings_category_selected: 0,
            codex_settings_selected: 0,
            sound_settings_selected: 0,
            telegram_selected_field: 0,
            telegram_editing: false,
            telegram_edit_buffer: String::new(),
            busy_animation_frame: 0,
            last_busy_animation_tick: Instant::now(),
            last_draw_elapsed: Duration::default(),
            frame_budget_exceeded: false,
            deferred_hook_events: Vec::new(),
            deferred_scan_result: None,
            notification_inbox: crate::notification_inbox::load(),
            notification_inbox_selected: 0,
            relay_config_last_poll_at: Instant::now(),
            relay_config_source_path: None,
            relay_config_source_modified_ms: None,
            relay_config_source_len: None,
            pending_external_relay_reload: false,
        }
    }

    pub fn apply_theme(&mut self, name: &str) {
        self.config.theme = name.to_string();
        self.theme = Theme::by_name(name);
        self.config.save();
        self.preview.theme_before_preview = None;
        self.clear_preview_render_caches();
        self.dirty = true;
    }

    pub fn preview_theme(&mut self, name: &str) {
        self.theme = Theme::by_name(name);
        self.clear_preview_render_caches();
        self.dirty = true;
    }

    pub fn clear_relay_popup_state(&mut self) {
        self.relay_popup_mode = RelayPopupMode::None;
        self.relay_popup_selected = 0;
        self.relay_popup_field = 0;
        self.relay_popup_editing = false;
        self.relay_popup_buffer.clear();
    }

    pub fn invalidate_sidebar_cache(&mut self) {
        self.sidebar.sidebar_folders_dirty = true;
        self.sidebar.visible_sidebar_items_dirty = true;
        self.sidebar.preferred_panel_width_cache = None;
    }

    pub fn invalidate_sidebar_visible_cache(&mut self) {
        self.sidebar.visible_sidebar_items_dirty = true;
        self.sidebar.preferred_panel_width_cache = None;
    }

    pub fn showing_live_sessions(&self) -> bool {
        self.sidebar.display_session_scope == "live"
    }

    pub fn thread_list_view(&self) -> ThreadListView {
        self.sidebar.thread_list_view
    }

    pub fn apply_display_session_scope(&mut self, scope: &str, persist_default: bool) -> bool {
        let normalized = if scope == "all" { "all" } else { "live" };
        let runtime_changed = self.sidebar.display_session_scope != normalized;
        let config_changed = self.config.display.session_scope != normalized;

        if persist_default && config_changed {
            self.config.display.session_scope = normalized.to_string();
            self.config.save();
        }

        if runtime_changed {
            self.sidebar.display_session_scope = normalized.to_string();
            self.sidebar.pending_thread_action = None;
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
        if self.thread_list_view() != ThreadListView::Normal {
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

pub(crate) fn new_handoff_trace(prefix: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{prefix}-{stamp}-{}", std::process::id())
}
