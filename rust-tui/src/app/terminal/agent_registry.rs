use std::path::PathBuf;

use crate::model::{AgentPanel, AgentState, AgentType};
use crate::terminal_runtime::TerminalError;

use super::{App, TerminalPaneId, TerminalProfile};

const NATIVE_AGENT_PANE_PREFIX: &str = "native:";

impl App {
    /// Resolves a live sidebar entry back to its PAD-owned terminal pane,
    /// including panes in background tabs.
    pub fn is_native_agent_terminal_id(live_pane_id: &str) -> bool {
        parse_native_agent_pane_id(live_pane_id).is_some()
    }

    pub fn focus_native_agent_terminal(
        &mut self,
        live_pane_id: &str,
    ) -> Result<bool, TerminalError> {
        let Some(pane_id) = parse_native_agent_pane_id(live_pane_id) else {
            return Ok(false);
        };
        let Some(tab_index) = self
            .terminal
            .workspace
            .tabs
            .iter()
            .position(|tab| tab.root.contains(pane_id))
        else {
            return Ok(false);
        };

        let mut workspace = self.terminal.workspace.clone();
        let tab_changed = workspace.focus_tab(tab_index);
        let pane_changed = workspace.focus_pane(pane_id);
        if workspace.focused_pane_id() != Some(pane_id) {
            return Ok(false);
        }
        if tab_changed || pane_changed {
            self.persist_terminal_workspace(&workspace)?;
            self.terminal.workspace = workspace;
        }
        self.mark_native_agent_panel_active(pane_id);
        let _ = self.focus_terminal();
        self.dirty = true;
        Ok(true)
    }

    pub fn close_native_agent_terminal(
        &mut self,
        live_pane_id: &str,
    ) -> Result<bool, TerminalError> {
        match parse_native_agent_pane_id(live_pane_id) {
            Some(pane_id) => self.close_terminal_pane(pane_id),
            None => Ok(false),
        }
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
            transcript_path: None,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
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

    pub(super) fn reindex_native_agent_panels(&mut self) {
        let indices = self
            .terminal
            .workspace
            .tabs
            .iter()
            .enumerate()
            .flat_map(|(tab_index, tab)| {
                tab.root
                    .pane_ids()
                    .into_iter()
                    .map(move |pane_id| (pane_id, tab_index + 1))
            })
            .collect::<Vec<_>>();
        let mut changed = false;
        for panel in &mut self.panels {
            let Some(pane_id) = parse_native_agent_pane_id(&panel.pane_id) else {
                continue;
            };
            let Some((_, tab_index)) = indices.iter().find(|(candidate, _)| *candidate == pane_id)
            else {
                continue;
            };
            let window_index = tab_index.to_string();
            changed |= panel.window_index != window_index;
            panel.window_index = window_index;
        }
        if changed {
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
        let mut focused_agent = false;
        for panel in &mut self.panels {
            if is_native_agent_pane_id(&panel.pane_id) {
                let is_active = panel.pane_id == live_pane_id;
                focused_agent |= is_active;
                changed |= panel.is_active != is_active;
                panel.is_active = is_active;
            }
        }
        if focused_agent {
            let selected = format!("live:{live_pane_id}");
            changed |= self.sidebar.selected_sidebar_key.as_deref() != Some(selected.as_str());
            self.sidebar.selected_sidebar_key = Some(selected);
        }
        if changed {
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
        }
    }
}

fn is_native_agent_pane_id(pane_id: &str) -> bool {
    parse_native_agent_pane_id(pane_id).is_some()
}

pub(super) fn native_agent_pane_id(pane_id: TerminalPaneId) -> String {
    format!("{NATIVE_AGENT_PANE_PREFIX}{}", pane_id.serial())
}

pub(super) fn parse_native_agent_pane_id(pane_id: &str) -> Option<TerminalPaneId> {
    let serial = pane_id
        .strip_prefix(NATIVE_AGENT_PANE_PREFIX)?
        .parse()
        .ok()?;
    Some(TerminalPaneId::new(serial))
}

pub(super) fn native_profile_agent_type(profile: TerminalProfile) -> Option<AgentType> {
    match profile {
        TerminalProfile::Codex => Some(AgentType::Codex),
        TerminalProfile::Claude => Some(AgentType::Claude),
        TerminalProfile::OpenCode => Some(AgentType::OpenCode),
        TerminalProfile::Shell | TerminalProfile::GithubCli => None,
    }
}
