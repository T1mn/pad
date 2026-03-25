pub mod actions;
pub mod async_ops;
pub mod navigation;
pub mod state;

use crate::fuzzy::FuzzyPicker;
use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{
    AgentPanel, AgentState, AgentStateSource, PreviewSource, PreviewTurn, SessionCacheState,
};
use crate::theme::{Config, Theme};
use crate::tree;
use async_ops::ScanResult;
use ratatui::widgets::TableState;
use state::{FocusTarget, Mode, RelayView};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
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
    pub preview_source: PreviewSource,
    pub preview_turns: Vec<PreviewTurn>,
    pub preview_selected_turn: Option<usize>,
    pub preview_expanded_turn: Option<usize>,
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
    pub hook_rx: Option<mpsc::Receiver<HookEvent>>,
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
    pub preview_focus: FocusTarget,
    pub preview_scroll: u16,
    pub preview_list_scroll: u16,
    pub preview_follow_bottom: bool,
    pub preview_follow_selection: bool,
    pub last_panel_tab_at: Option<Instant>,
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
    pub provider_test_rx: Option<mpsc::Receiver<(usize, usize, bool, String)>>,
    // Agent style settings
    pub agent_style_selected: usize,
    pub busy_animation_frame: usize,
    pub last_busy_animation_tick: Instant,
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
            preview_source: PreviewSource::Tmux,
            preview_turns: Vec::new(),
            preview_selected_turn: None,
            preview_expanded_turn: None,
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
            hook_rx: None,
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
            preview_focus: FocusTarget::Panel,
            preview_scroll: 0,
            preview_list_scroll: 0,
            preview_follow_bottom: true,
            preview_follow_selection: true,
            last_panel_tab_at: None,
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
            busy_animation_frame: 0,
            last_busy_animation_tick: Instant::now(),
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

    pub fn invalidate_preview(&mut self) {
        self.last_preview_update = Instant::now() - Duration::from_secs(1);
    }

    pub fn preview_is_focused(&self) -> bool {
        self.preview_focus == FocusTarget::Preview && !self.show_tree
    }

    pub fn toggle_preview_focus(&mut self) -> bool {
        if self.show_tree || self.selected_panel().is_none() {
            return false;
        }
        self.preview_focus = match self.preview_focus {
            FocusTarget::Panel => FocusTarget::Preview,
            FocusTarget::Preview => FocusTarget::Panel,
        };
        self.clear_unread_stop_for_selected_panel();
        self.dirty = true;
        true
    }

    pub fn focus_panel(&mut self) {
        if self.preview_focus != FocusTarget::Panel {
            self.preview_focus = FocusTarget::Panel;
        }
        self.clear_unread_stop_for_selected_panel();
        self.dirty = true;
    }

    pub fn has_session_preview_turns(&self) -> bool {
        self.preview_source == PreviewSource::Session && !self.preview_turns.is_empty()
    }

    pub fn note_panel_tab(&mut self) {
        self.last_panel_tab_at = Some(Instant::now());
    }

    pub fn recent_panel_tab_within(&self, window: Duration) -> bool {
        self.last_panel_tab_at
            .map(|instant| instant.elapsed() <= window)
            .unwrap_or(false)
    }

    pub fn clear_panel_tab(&mut self) {
        self.last_panel_tab_at = None;
    }

    fn preview_uses_list_scroll(&self) -> bool {
        self.has_session_preview_turns() && self.preview_expanded_turn.is_none()
    }

    pub fn open_latest_preview_turn(&mut self) -> bool {
        if self.show_tree {
            return false;
        }

        let Some(panel) = self.selected_panel().cloned() else {
            return false;
        };

        let same_session_preview = self.preview_source == PreviewSource::Session
            && self.preview_pane_id.as_deref() == Some(panel.pane_id.as_str())
            && !self.preview_turns.is_empty();

        if !same_session_preview {
            self.preview_turns = panel.cached_preview_turns.clone();
            self.preview_pane_id = Some(panel.pane_id.clone());
            if !self.preview_turns.is_empty() {
                self.preview_source = PreviewSource::Session;
            }
        }

        if !self.has_session_preview_turns() {
            return false;
        }

        self.preview_focus = FocusTarget::Preview;
        self.preview_selected_turn = Some(0);
        self.preview_expanded_turn = Some(0);
        self.preview_scroll = 0;
        self.preview_list_scroll = 0;
        self.preview_follow_bottom = false;
        self.preview_follow_selection = true;
        self.dirty = true;
        true
    }

    pub fn select_next_preview_turn(&mut self) -> bool {
        if !self.has_session_preview_turns() {
            return false;
        }
        let max = self.preview_turns.len().saturating_sub(1);
        let next = match self.preview_selected_turn {
            Some(idx) => (idx + 1).min(max),
            None => 0,
        };
        self.preview_selected_turn = Some(next);
        if self.preview_expanded_turn.is_some() {
            self.preview_expanded_turn = Some(next);
            self.preview_scroll = 0;
        }
        self.preview_follow_bottom = false;
        self.preview_follow_selection = true;
        self.dirty = true;
        true
    }

    pub fn select_previous_preview_turn(&mut self) -> bool {
        if !self.has_session_preview_turns() {
            return false;
        }
        let prev = match self.preview_selected_turn {
            Some(idx) => idx.saturating_sub(1),
            None => 0,
        };
        self.preview_selected_turn = Some(prev);
        if self.preview_expanded_turn.is_some() {
            self.preview_expanded_turn = Some(prev);
            self.preview_scroll = 0;
        }
        self.preview_follow_bottom = false;
        self.preview_follow_selection = true;
        self.dirty = true;
        true
    }

    pub fn step_back_preview_focus(&mut self) -> bool {
        if self.preview_expanded_turn.is_some() {
            self.preview_expanded_turn = None;
            self.preview_scroll = 0;
            self.preview_follow_selection = true;
            self.dirty = true;
            return true;
        }
        if self.preview_selected_turn.is_some() {
            self.preview_selected_turn = None;
            self.preview_follow_selection = false;
            self.dirty = true;
            return true;
        }
        if self.preview_is_focused() {
            self.preview_focus = FocusTarget::Panel;
            self.clear_unread_stop_for_selected_panel();
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn toggle_preview_turn_expanded(&mut self) -> bool {
        if !self.has_session_preview_turns() {
            return false;
        }
        let Some(selected) = self.preview_selected_turn else {
            return false;
        };
        if self.preview_expanded_turn == Some(selected) {
            self.preview_expanded_turn = None;
        } else {
            self.preview_expanded_turn = Some(selected);
        }
        self.preview_scroll = 0;
        self.preview_follow_bottom = false;
        self.preview_follow_selection = true;
        self.dirty = true;
        true
    }

    pub fn scroll_preview_by(&mut self, delta: i32) {
        if self.preview_uses_list_scroll() {
            self.preview_follow_selection = false;
            if delta >= 0 {
                self.preview_list_scroll = self.preview_list_scroll.saturating_add(delta as u16);
            } else {
                self.preview_list_scroll = self.preview_list_scroll.saturating_sub((-delta) as u16);
            }
        } else {
            self.preview_follow_bottom = false;
            if delta >= 0 {
                self.preview_scroll = self.preview_scroll.saturating_add(delta as u16);
            } else {
                self.preview_scroll = self.preview_scroll.saturating_sub((-delta) as u16);
            }
        }
        self.dirty = true;
    }

    pub fn scroll_preview_to_top(&mut self) {
        if self.preview_uses_list_scroll() {
            self.preview_list_scroll = 0;
            self.preview_follow_selection = false;
        } else {
            self.preview_scroll = 0;
            self.preview_follow_bottom = false;
        }
        self.dirty = true;
    }

    pub fn scroll_preview_to_bottom(&mut self) {
        if self.preview_uses_list_scroll() {
            self.preview_list_scroll = u16::MAX;
            self.preview_follow_selection = false;
        } else {
            self.preview_follow_bottom = true;
        }
        self.dirty = true;
    }

    pub fn apply_hook_event(&mut self, event: HookEvent) {
        let pane_id = match event.tmux.pane_id.clone() {
            Some(id) => id,
            None => return,
        };
        let panel_item_focused = self.panel_item_is_focused(&pane_id);

        let should_refresh_preview = self
            .selected_panel()
            .map(|panel| panel.pane_id == pane_id)
            .unwrap_or(false);

        let mut persisted_snapshot = None;

        if let Some(panel) = self.panels.iter_mut().find(|p| p.pane_id == pane_id) {
            if event.session_id.is_some() {
                panel.agent_session_id = event.session_id.clone();
            }
            if event.transcript_path.is_some() {
                panel.transcript_path = event.transcript_path.clone();
            }
            match event.event.as_str() {
                "session_start" => {}
                "user_prompt_submit" => {
                    panel.state = AgentState::Busy;
                    panel.state_source = AgentStateSource::Hook;
                    panel.is_active = true;
                    panel.last_user_prompt = event.prompt.clone();
                    panel.has_unread_stop = false;
                }
                "stop" => {
                    panel.state = AgentState::Waiting;
                    panel.state_source = AgentStateSource::Hook;
                    panel.is_active = false;
                    panel.has_unread_stop = !panel_item_focused;
                    if event.last_assistant_message.is_some() {
                        panel.last_assistant_message = event.last_assistant_message.clone();
                    }
                }
                _ => {}
            }

            match crate::session_cache::persist_hook_event(panel, &event) {
                Ok(snapshot) => persisted_snapshot = snapshot,
                Err(err) => log_debug!("session_cache: persist hook failed: {}", err),
            }

            if let Some(snapshot) = persisted_snapshot.as_ref() {
                panel.agent_session_id = Some(snapshot.agent_session_id.clone());
                panel.transcript_path = snapshot.transcript_path.clone();
                panel.cached_preview_turns = snapshot.recent_turns.clone();
                panel.last_user_prompt = snapshot.last_user_prompt.clone();
                panel.last_assistant_message = snapshot.last_assistant_message.clone();
                panel.session_cache_state = Some(SessionCacheState::Confirmed);
            }

            if should_refresh_preview {
                self.invalidate_preview();
            }
            self.dirty = true;
        }
    }

    fn panel_item_is_focused(&self, pane_id: &str) -> bool {
        !self.show_tree
            && self.preview_focus == FocusTarget::Panel
            && self
                .selected_panel()
                .map(|panel| panel.pane_id == pane_id)
                .unwrap_or(false)
    }

    pub fn clear_unread_stop_for_selected_panel(&mut self) {
        if self.show_tree || self.preview_focus != FocusTarget::Panel {
            return;
        }

        let Some(selected_pane_id) = self.selected_panel().map(|panel| panel.pane_id.clone())
        else {
            return;
        };

        if let Some(panel) = self
            .panels
            .iter_mut()
            .find(|panel| panel.pane_id == selected_pane_id)
        {
            panel.has_unread_stop = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::{HookEvent, HookTmuxInfo};
    use crate::model::{AgentPanel, AgentType, PreviewTurn};

    #[test]
    fn open_latest_preview_turn_uses_selected_panel_cached_turns() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "main".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            is_active: true,
            state: AgentState::Busy,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("- item".into()),
            }],
            session_cache_state: Some(SessionCacheState::Cached),
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });

        app.preview_source = PreviewSource::Session;
        app.preview_pane_id = Some("%other".into());
        app.preview_turns = vec![PreviewTurn {
            question: "stale".into(),
            answer: Some("stale".into()),
        }];

        assert!(app.open_latest_preview_turn());
        assert_eq!(app.preview_pane_id.as_deref(), Some("%1"));
        assert_eq!(app.preview_selected_turn, Some(0));
        assert_eq!(app.preview_expanded_turn, Some(0));
        assert_eq!(app.preview_turns[0].question, "latest");
    }

    fn stop_event(pane_id: &str) -> HookEvent {
        HookEvent {
            event: "stop".into(),
            session_id: Some("session-1".into()),
            transcript_path: None,
            cwd: None,
            prompt: None,
            last_assistant_message: Some("done".into()),
            timestamp: None,
            tmux: HookTmuxInfo {
                pane_id: Some(pane_id.into()),
                session_name: Some("0".into()),
                window_index: Some("1".into()),
                pane_index: Some("1".into()),
                pane_current_path: Some("/tmp/demo".into()),
            },
        }
    }

    #[test]
    fn stop_hook_marks_panel_unread_when_panel_item_is_not_focused() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "main".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            is_active: true,
            state: AgentState::Busy,
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
        });
        app.table_state.select(Some(0));
        app.preview_focus = FocusTarget::Preview;

        app.apply_hook_event(stop_event("%1"));

        assert!(app.panels[0].has_unread_stop);
    }

    #[test]
    fn focusing_panel_clears_unread_stop_marker() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "main".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            is_active: false,
            state: AgentState::Waiting,
            state_source: AgentStateSource::Hook,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: Some("done".into()),
            has_unread_stop: true,
        });
        app.table_state.select(Some(0));
        app.preview_focus = FocusTarget::Preview;

        app.focus_panel();

        assert!(!app.panels[0].has_unread_stop);
    }
}
