pub mod actions;
pub mod async_ops;
pub mod navigation;
pub mod state;

use crate::model::AgentPanel;
use crate::theme::{Config, Theme};
use crate::tree;
use async_ops::ScanResult;
use ratatui::widgets::TableState;
use state::Mode;
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
    pub theme: Theme,
    pub theme_selector_open: bool,
    pub settings_selected: usize,
    pub theme_selected: usize,
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
    pub saved_tmux_bindings: Vec<String>,
}

impl App {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let config = Config::load();
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
            theme,
            theme_selector_open: false,
            settings_selected: 0,
            theme_selected: 0,
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
            saved_tmux_bindings: Vec::new(),
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
