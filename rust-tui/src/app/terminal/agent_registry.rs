use std::path::PathBuf;

use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

use super::{App, TerminalPaneId};

const NATIVE_AGENT_PANE_PREFIX: &str = "native:";

impl App {
    /// Resolves a live sidebar entry back to its PAD-owned terminal pane,
    /// including panes in background tabs.
    pub fn is_native_agent_terminal_id(live_pane_id: &str) -> bool {
        parse_native_agent_pane_id(live_pane_id).is_some()
    }

    pub fn focus_native_agent_terminal(&mut self, live_pane_id: &str) -> bool {
        let Some(pane_id) = parse_native_agent_pane_id(live_pane_id) else {
            return false;
        };
        let Some(tab_index) = self
            .terminal
            .workspace
            .tabs
            .iter()
            .position(|tab| tab.root.contains(pane_id))
        else {
            return false;
        };

        let tab_changed = self.terminal.workspace.focus_tab(tab_index);
        let pane_changed = self.terminal.workspace.focus_pane(pane_id);
        if self.focused_terminal_pane_id() != Some(pane_id) {
            return false;
        }
        self.mark_native_agent_panel_active(pane_id);
        let _ = self.focus_terminal();
        self.dirty = true;
        if tab_changed || pane_changed {
            self.persist_terminal_workspace();
        }
        true
    }

    pub fn close_native_agent_terminal(&mut self, live_pane_id: &str) -> bool {
        parse_native_agent_pane_id(live_pane_id)
            .is_some_and(|pane_id| self.close_terminal_pane(pane_id))
    }

    pub(super) fn register_native_agent_panel(
        &mut self,
        pane_id: TerminalPaneId,
        agent_type: AgentType,
        label: String,
        cwd: PathBuf,
    ) {
        let live_pane_id = native_agent_pane_id(pane_id);
        let working_dir = cwd.to_string_lossy().into_owned();
        let window_index = self
            .terminal
            .workspace
            .tabs
            .iter()
            .position(|tab| tab.root.contains(pane_id))
            .map(|index| index + 1)
            .unwrap_or_default();
        for panel in &mut self.panels {
            if is_native_agent_pane_id(&panel.pane_id) {
                panel.is_active = false;
            }
        }
        self.panels.push(AgentPanel {
            session: "native".to_string(),
            window: label,
            window_index: window_index.to_string(),
            pane: pane_id.to_string(),
            pane_id: live_pane_id.clone(),
            agent_type,
            working_dir: working_dir.clone(),
            is_active: true,
            state: AgentState::Idle,
            state_source: AgentStateSource::Native,
            transcript_path: None,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: Some(std::time::Instant::now()),
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });
        self.sidebar.expanded_folders.insert(working_dir);
        self.sidebar.selected_sidebar_key = Some(format!("live:{live_pane_id}"));
        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
    }

    pub(super) fn remove_native_agent_panel(&mut self, pane_id: TerminalPaneId) {
        let live_pane_id = native_agent_pane_id(pane_id);
        let original_len = self.panels.len();
        self.panels.retain(|panel| panel.pane_id != live_pane_id);
        if self.panels.len() != original_len {
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
        }
    }

    pub(super) fn remove_all_native_agent_panels(&mut self) -> bool {
        let original_len = self.panels.len();
        self.panels
            .retain(|panel| !is_native_agent_pane_id(&panel.pane_id));
        self.panels.len() != original_len
    }

    pub(super) fn mark_native_agent_panel_active(&mut self, pane_id: TerminalPaneId) {
        let live_pane_id = native_agent_pane_id(pane_id);
        let mut changed = false;
        for panel in &mut self.panels {
            if is_native_agent_pane_id(&panel.pane_id) {
                let is_active = panel.pane_id == live_pane_id;
                changed |= panel.is_active != is_active;
                panel.is_active = is_active;
            }
        }
        if changed {
            self.invalidate_sidebar_cache();
        }
    }
}

fn is_native_agent_pane_id(pane_id: &str) -> bool {
    parse_native_agent_pane_id(pane_id).is_some()
}

pub(super) fn native_agent_pane_id(pane_id: TerminalPaneId) -> String {
    format!("{NATIVE_AGENT_PANE_PREFIX}{}", pane_id.serial())
}

fn parse_native_agent_pane_id(pane_id: &str) -> Option<TerminalPaneId> {
    let serial = pane_id
        .strip_prefix(NATIVE_AGENT_PANE_PREFIX)?
        .parse()
        .ok()?;
    Some(TerminalPaneId::new(serial))
}
