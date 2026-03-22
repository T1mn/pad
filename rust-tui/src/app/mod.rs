pub mod actions;
pub mod async_ops;
pub mod navigation;
pub mod state;

use crate::fuzzy::FuzzyPicker;
use crate::model::AgentPanel;
use crate::theme::{Config, Theme};
use crate::tree;
use async_ops::ScanResult;
use ratatui::widgets::TableState;
use state::{Mode, RelayView};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;

/// Application state
pub struct App {
    pub panels: Vec<AgentPanel>,
    pub table_state: TableState,
    pub mode: Mode,
    pub last_refresh: Instant,
    pub search_query: String,
    pub is_searching: bool,
    pub preview_content: String,
    pub preview_pane_id: Option<String>,
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
    pub preview_rx: Option<mpsc::Receiver<(String, String)>>,
    pub last_preview_update: Instant,
    pub refresh_after_attach: bool,
    pub should_quit: bool,
    pub dirty: bool,
    pub show_tree: bool,
    pub file_tree: Option<tree::FileTree>,
    pub agent_launcher: Option<tree::AgentLauncher>,
    pub delete_target: Option<AgentPanel>,
    pub theme_before_preview: Option<String>,
    pub file_preview_content: String,
    pub file_preview_path: Option<PathBuf>,
    pub file_preview_scroll: u16,
    pub preview_scroll: u16,
    pub same_session_attached: bool,
    pub pending_status_restore: bool,
    pub saved_tmux_bindings: Vec<String>,
    pub fuzzy_picker: Option<FuzzyPicker>,
    /// Whether the fuzzy picker was opened from Normal mode (for 'c' key flow)
    pub fuzzy_from_normal: bool,
    // Relay settings state
    pub relay_selected_agent: usize,
    pub relay_selected_provider: usize,
    pub relay_editing: bool,
    pub relay_edit_field: usize,   // 0=label, 1=base_url, 2=api_key
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
    pub provider_test_rx: Option<mpsc::Receiver<(usize, usize, bool, String)>>,
}

impl App {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let config = Config::load();
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
            refresh_after_attach: false,
            should_quit: false,
            dirty: true,
            show_tree: false,
            file_tree: None,
            agent_launcher: None,
            delete_target: None,
            theme_before_preview: None,
            file_preview_content: String::new(),
            file_preview_path: None,
            file_preview_scroll: 0,
            preview_scroll: 0,
            same_session_attached: false,
            pending_status_restore: false,
            saved_tmux_bindings: Vec::new(),
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
        }
    }

    pub fn apply_theme(&mut self, name: &str) {
        self.config.theme = name.to_string();
        self.theme = Theme::by_name(name);
        self.config.save();
        self.theme_before_preview = None;
        self.dirty = true;
    }

    pub fn preview_theme(&mut self, name: &str) {
        self.theme = Theme::by_name(name);
        self.dirty = true;
    }
}
