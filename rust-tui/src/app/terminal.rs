mod agent_registry;
mod controller_io;
mod interaction {
    use super::{model, App, TerminalError, TerminalInteractionState};

    impl App {
        pub fn terminal_interaction(&self) -> &TerminalInteractionState {
            self.terminal.interaction()
        }

        pub fn enter_terminal_command_layer(&mut self) -> bool {
            if !self.terminal.is_active() {
                return false;
            }
            self.terminal.interaction = TerminalInteractionState::Command;
            self.dirty = true;
            true
        }

        pub fn cancel_terminal_command_layer(&mut self) {
            self.terminal.interaction = TerminalInteractionState::Direct;
            self.dirty = true;
        }

        pub fn begin_terminal_rename(&mut self) -> bool {
            let Some((pane_id, label)) = self.focused_terminal_pane().and_then(|pane| {
                self.focused_terminal_pane_id()
                    .map(|pane_id| (pane_id, pane.label().to_string()))
            }) else {
                return false;
            };
            self.terminal.interaction = TerminalInteractionState::Rename {
                pane_id,
                buffer: label,
            };
            self.dirty = true;
            true
        }

        pub fn append_terminal_rename_text(&mut self, text: &str) -> bool {
            let TerminalInteractionState::Rename { buffer, .. } = &mut self.terminal.interaction
            else {
                return false;
            };
            let remaining = model::MAX_TERMINAL_LABEL_CHARS.saturating_sub(buffer.chars().count());
            buffer.extend(
                text.chars()
                    .filter(|character| !character.is_control())
                    .take(remaining),
            );
            self.dirty = true;
            true
        }

        pub fn backspace_terminal_rename(&mut self) -> bool {
            let TerminalInteractionState::Rename { buffer, .. } = &mut self.terminal.interaction
            else {
                return false;
            };
            let changed = buffer.pop().is_some();
            self.dirty |= changed;
            changed
        }

        pub fn clear_terminal_rename(&mut self) -> bool {
            let TerminalInteractionState::Rename { buffer, .. } = &mut self.terminal.interaction
            else {
                return false;
            };
            let changed = !buffer.is_empty();
            buffer.clear();
            self.dirty |= changed;
            changed
        }

        pub fn commit_terminal_rename(&mut self) -> Result<bool, TerminalError> {
            let state = std::mem::take(&mut self.terminal.interaction);
            let TerminalInteractionState::Rename { pane_id, buffer } = state else {
                return Ok(false);
            };
            match self.rename_terminal_pane(pane_id, &buffer) {
                Ok(changed) => {
                    self.terminal.interaction = TerminalInteractionState::Direct;
                    self.dirty = true;
                    Ok(changed)
                }
                Err(error) => {
                    self.terminal.interaction =
                        TerminalInteractionState::Rename { pane_id, buffer };
                    self.dirty = true;
                    Err(error)
                }
            }
        }

        pub fn terminal_is_active(&self) -> bool {
            self.terminal.is_active()
        }

        pub fn terminal_is_focused(&self) -> bool {
            self.terminal.is_active()
                && self.focused_terminal_pane_id().is_some()
                && self.mode == crate::app::state::Mode::Normal
                && self.preview_is_focused()
        }

        pub fn focus_terminal(&mut self) -> bool {
            if !self.terminal.is_active()
                || self.focused_terminal_pane_id().is_none()
                || self.sidebar.show_tree
            {
                return false;
            }
            self.preview.focus = crate::app::state::FocusTarget::Preview;
            self.dirty = true;
            true
        }
    }
}
mod model;
mod runtime_io;

#[cfg(test)]
pub(crate) mod tests;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::model::AgentType;
use crate::terminal_runtime::{
    PaneFrame, TerminalController, TerminalError, TerminalFrameReader, TerminalMode, TerminalSize,
    TransportExit,
};

use controller_io::TerminalPaneRuntime;
pub use model::{
    TerminalCommandDefinition, TerminalLayoutNode, TerminalPaneDefinition, TerminalPaneId,
    TerminalProfile, TerminalSplitAxis, TerminalTab, TerminalWorkspace,
    DEFAULT_SPLIT_RATIO_PER_MILLE,
};

use super::App;

const MAX_PENDING_INPUT_BYTES: usize = 1024 * 1024;
const MAX_PENDING_SCROLL_COMMANDS: usize = 128;
const MIN_SPLIT_CHILD_COLUMNS: u16 = 4;
const MIN_SPLIT_CHILD_LINES: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalPaneLifecycle {
    Opening,
    Running,
    Exited,
    Failed,
    Closing,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum TerminalInteractionState {
    #[default]
    Direct,
    Command,
    Rename {
        pane_id: TerminalPaneId,
        buffer: String,
    },
}

#[derive(Default)]
pub struct TerminalUiState {
    controller: Option<TerminalController>,
    frames: Option<TerminalFrameReader>,
    workspace: TerminalWorkspace,
    panes: HashMap<TerminalPaneId, TerminalPaneRuntime>,
    flush_order: Vec<TerminalPaneId>,
    flush_cursor: usize,
    interaction: TerminalInteractionState,
}

#[derive(Clone, Copy)]
pub struct TerminalPaneView<'a> {
    definition: &'a TerminalPaneDefinition,
    runtime: Option<&'a TerminalPaneRuntime>,
}

impl<'a> TerminalPaneView<'a> {
    pub fn label(&self) -> &'a str {
        &self.definition.label
    }

    pub fn cwd(&self) -> &'a Path {
        &self.definition.cwd
    }

    pub fn lifecycle(&self) -> TerminalPaneLifecycle {
        self.runtime
            .map(|runtime| runtime.lifecycle)
            .unwrap_or(TerminalPaneLifecycle::Opening)
    }

    pub fn frame(&self) -> Option<&'a Arc<PaneFrame>> {
        self.runtime.and_then(|runtime| runtime.frame.as_ref())
    }

    pub fn mode(&self) -> TerminalMode {
        self.runtime
            .map(TerminalPaneRuntime::mode)
            .unwrap_or_default()
    }

    pub fn size(&self) -> Option<TerminalSize> {
        self.runtime.and_then(|runtime| {
            runtime
                .pending_resize
                .or(runtime.last_requested_size)
                .or_else(|| runtime.pending_open.as_ref().map(|request| request.size))
                .or_else(|| runtime.frame.as_ref().map(|frame| frame.terminal.size))
        })
    }

    pub fn error(&self) -> Option<&'a str> {
        self.runtime.and_then(|runtime| runtime.error.as_deref())
    }

    pub fn exit(&self) -> Option<TransportExit> {
        self.runtime.and_then(|runtime| runtime.exit)
    }
}

impl TerminalUiState {
    pub fn is_active(&self) -> bool {
        self.controller.is_some()
    }

    pub fn workspace(&self) -> &TerminalWorkspace {
        &self.workspace
    }

    pub fn pane(&self, pane_id: TerminalPaneId) -> Option<TerminalPaneView<'_>> {
        let definition = self.workspace.pane(pane_id)?;
        Some(TerminalPaneView {
            definition,
            runtime: self.panes.get(&pane_id),
        })
    }

    pub fn focused_pane(&self) -> Option<TerminalPaneView<'_>> {
        self.pane(self.workspace.focused_pane_id()?)
    }

    pub fn interaction(&self) -> &TerminalInteractionState {
        &self.interaction
    }

    pub fn frame(&self) -> Option<&Arc<PaneFrame>> {
        self.focused_pane().and_then(|pane| pane.frame())
    }

    pub fn mode(&self) -> TerminalMode {
        self.focused_pane()
            .map(|pane| pane.mode())
            .unwrap_or_default()
    }

    pub fn error(&self) -> Option<&str> {
        self.focused_pane().and_then(|pane| pane.error())
    }

    pub fn exit(&self) -> Option<TransportExit> {
        self.focused_pane().and_then(|pane| pane.exit())
    }
}

impl App {
    /// Starts one shared native terminal controller. A retained workspace is
    /// relaunched after shutdown; otherwise a default shell tab is created.
    pub fn start_native_terminal(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
        if self.terminal.is_active() {
            return Ok(());
        }
        if !self.terminal.workspace.tabs.is_empty() {
            return self.restore_native_terminal_workspace(self.terminal.workspace.clone(), size);
        }
        self.terminal.start_controller()?;
        self.create_terminal_tab(TerminalProfile::Shell, size)?;
        Ok(())
    }

    /// Restores a serializable layout by launching fresh PTYs for its pane
    /// definitions. Runtime IDs, epochs, frames, and queues are never restored.
    pub fn restore_native_terminal_workspace(
        &mut self,
        mut workspace: TerminalWorkspace,
        initial_size: TerminalSize,
    ) -> Result<(), TerminalError> {
        if self.terminal.is_active() {
            return Err(TerminalError::new(
                "native terminal workspace is already running",
            ));
        }
        workspace
            .normalize_after_restore()
            .map_err(TerminalError::new)?;
        self.terminal.start_controller()?;
        let definitions = workspace.panes.clone();
        self.terminal.workspace = workspace;
        self.remove_all_native_agent_panels();
        for definition in &definitions {
            let runtime_definition = self.runtime_terminal_definition(definition.clone());
            self.terminal
                .install_pane_runtime(&runtime_definition, initial_size);
            if let Some(agent_type) = agent_registry::native_profile_agent_type(definition.profile)
            {
                self.register_native_agent_panel(
                    definition.id,
                    agent_type,
                    definition.label.clone(),
                    definition.cwd.clone(),
                );
            }
        }
        self.terminal.flush_commands();
        if let Some(focused) = self.focused_terminal_pane_id() {
            self.mark_native_agent_panel_active(focused);
        }
        self.dirty = true;
        Ok(())
    }

    pub fn terminal_workspace_snapshot(&self) -> TerminalWorkspace {
        self.terminal.workspace.clone()
    }

    pub fn terminal_workspace(&self) -> &TerminalWorkspace {
        self.terminal.workspace()
    }

    pub fn terminal_pane(&self, pane_id: TerminalPaneId) -> Option<TerminalPaneView<'_>> {
        self.terminal.pane(pane_id)
    }

    pub fn focused_terminal_pane(&self) -> Option<TerminalPaneView<'_>> {
        self.terminal.focused_pane()
    }

    pub fn focused_terminal_pane_id(&self) -> Option<TerminalPaneId> {
        self.terminal.workspace.focused_pane_id()
    }

    pub fn create_terminal_tab(
        &mut self,
        profile: TerminalProfile,
        size: TerminalSize,
    ) -> Result<TerminalPaneId, TerminalError> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.create_terminal_tab_at(profile, cwd, size)
    }

    pub fn create_terminal_tab_at(
        &mut self,
        profile: TerminalProfile,
        cwd: PathBuf,
        size: TerminalSize,
    ) -> Result<TerminalPaneId, TerminalError> {
        self.ensure_terminal_controller()?;
        let mut workspace = self.terminal.workspace.clone();
        let pane_id = workspace.add_tab(profile, cwd).ok_or_else(|| {
            TerminalError::new(format!(
                "terminal workspace is limited to {} tabs and {} panes",
                model::MAX_TERMINAL_TABS,
                model::MAX_TERMINAL_PANES
            ))
        })?;
        let definition = workspace
            .pane(pane_id)
            .expect("new terminal tab has a pane definition")
            .clone();
        self.persist_terminal_workspace(&workspace)?;
        self.terminal.workspace = workspace;
        let runtime_definition = self.runtime_terminal_definition(definition.clone());
        self.terminal
            .install_pane_runtime(&runtime_definition, size);
        self.terminal.flush_commands();
        if let Some(agent_type) = agent_registry::native_profile_agent_type(profile) {
            self.register_native_agent_panel(pane_id, agent_type, definition.label, definition.cwd);
        } else {
            self.mark_native_agent_panel_active(pane_id);
        }
        self.dirty = true;
        Ok(pane_id)
    }

    /// Opens a shell tab and replaces that shell with a configured agent
    /// command. The command is runtime-only: workspace restore keeps the
    /// labelled shell but never auto-runs arbitrary text loaded from disk.
    pub fn launch_native_agent_terminal_at(
        &mut self,
        label: &str,
        command: &str,
        agent_type: AgentType,
        cwd: PathBuf,
        size: TerminalSize,
    ) -> Result<TerminalPaneId, TerminalError> {
        let command = command.trim();
        if command.is_empty()
            || command
                .chars()
                .any(|character| matches!(character, '\0' | '\r' | '\n'))
        {
            return Err(TerminalError::new(
                "native agent command must be one non-empty shell line",
            ));
        }
        let label = model::normalize_label(label).map_err(TerminalError::new)?;
        self.ensure_terminal_controller()?;
        let mut workspace = self.terminal.workspace.clone();
        let pane_id = workspace
            .add_tab(TerminalProfile::Shell, cwd.clone())
            .ok_or_else(|| {
                TerminalError::new(format!(
                    "terminal workspace is limited to {} tabs and {} panes",
                    model::MAX_TERMINAL_TABS,
                    model::MAX_TERMINAL_PANES
                ))
            })?;
        let definition = workspace
            .panes
            .iter_mut()
            .find(|pane| pane.id == pane_id)
            .expect("new terminal tab has a pane definition");
        definition.label = label.clone();
        self.persist_terminal_workspace(&workspace)?;
        let mut definition = workspace
            .pane(pane_id)
            .expect("persisted terminal tab has a pane definition")
            .clone();
        definition.command = native_agent_command(command);
        self.terminal.workspace = workspace;
        let runtime_definition = self.runtime_terminal_definition(definition.clone());
        self.terminal
            .install_pane_runtime(&runtime_definition, size);
        self.terminal.flush_commands();
        self.register_native_agent_panel(pane_id, agent_type, label, cwd);
        self.dirty = true;
        Ok(pane_id)
    }

    pub fn split_focused_terminal(
        &mut self,
        axis: TerminalSplitAxis,
        profile: TerminalProfile,
        size: TerminalSize,
    ) -> Result<TerminalPaneId, TerminalError> {
        let cwd = self
            .focused_terminal_pane()
            .map(|pane| pane.cwd().to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        self.split_focused_terminal_at(axis, profile, cwd, size)
    }

    pub fn split_focused_terminal_at(
        &mut self,
        axis: TerminalSplitAxis,
        profile: TerminalProfile,
        cwd: PathBuf,
        size: TerminalSize,
    ) -> Result<TerminalPaneId, TerminalError> {
        self.ensure_terminal_controller()?;
        validate_split_size(axis, size)?;
        let mut workspace = self.terminal.workspace.clone();
        let pane_id = workspace
            .split_focused(axis, profile, cwd)
            .ok_or_else(|| TerminalError::new("terminal workspace has no focused pane to split"))?;
        let definition = workspace
            .pane(pane_id)
            .expect("new terminal split has a pane definition")
            .clone();
        self.persist_terminal_workspace(&workspace)?;
        self.terminal.workspace = workspace;
        let runtime_definition = self.runtime_terminal_definition(definition.clone());
        self.terminal
            .install_pane_runtime(&runtime_definition, size);
        self.terminal.flush_commands();
        if let Some(agent_type) = agent_registry::native_profile_agent_type(profile) {
            self.register_native_agent_panel(pane_id, agent_type, definition.label, definition.cwd);
        } else {
            self.mark_native_agent_panel_active(pane_id);
        }
        self.dirty = true;
        Ok(pane_id)
    }

    pub fn close_terminal_pane(&mut self, pane_id: TerminalPaneId) -> Result<bool, TerminalError> {
        let mut workspace = self.terminal.workspace.clone();
        if !workspace.close_pane(pane_id) {
            return Ok(false);
        }
        self.persist_terminal_workspace(&workspace)?;
        self.terminal.workspace = workspace;
        self.terminal.queue_close(pane_id);
        self.remove_native_agent_panel(pane_id);
        self.reindex_native_agent_panels();
        if let Some(focused) = self.focused_terminal_pane_id() {
            self.mark_native_agent_panel_active(focused);
        } else {
            self.cancel_terminal_command_layer();
            self.focus_panel();
        }
        self.dirty = true;
        Ok(true)
    }

    pub fn close_focused_terminal(&mut self) -> Result<bool, TerminalError> {
        match self.focused_terminal_pane_id() {
            Some(pane_id) => self.close_terminal_pane(pane_id),
            None => Ok(false),
        }
    }

    pub fn rename_terminal_pane(
        &mut self,
        pane_id: TerminalPaneId,
        label: &str,
    ) -> Result<bool, TerminalError> {
        let label = model::normalize_label(label).map_err(TerminalError::new)?;
        let mut workspace = self.terminal.workspace.clone();
        if !workspace.rename_pane(pane_id, label.clone()) {
            return Ok(false);
        }
        self.persist_terminal_workspace(&workspace)?;
        self.terminal.workspace = workspace;
        self.terminal.queue_label(pane_id, label.clone());
        let live_pane_id = agent_registry::native_agent_pane_id(pane_id);
        if let Some(panel) = self
            .panels
            .iter_mut()
            .find(|panel| panel.pane_id == live_pane_id)
        {
            panel.window = label;
            self.invalidate_sidebar_cache();
        }
        self.dirty = true;
        Ok(true)
    }

    pub fn focus_terminal_pane(&mut self, pane_id: TerminalPaneId) -> Result<bool, TerminalError> {
        let mut workspace = self.terminal.workspace.clone();
        if !workspace.focus_pane(pane_id) {
            return Ok(false);
        }
        self.persist_terminal_workspace(&workspace)?;
        self.terminal.workspace = workspace;
        self.mark_native_agent_panel_active(pane_id);
        self.dirty = true;
        Ok(true)
    }

    pub fn cycle_terminal_pane(&mut self, delta: isize) -> Result<bool, TerminalError> {
        let mut workspace = self.terminal.workspace.clone();
        if !workspace.cycle_pane(delta) {
            return Ok(false);
        }
        self.persist_terminal_workspace(&workspace)?;
        self.terminal.workspace = workspace;
        if let Some(pane_id) = self.focused_terminal_pane_id() {
            self.mark_native_agent_panel_active(pane_id);
        }
        self.dirty = true;
        Ok(true)
    }

    pub fn cycle_terminal_tab(&mut self, delta: isize) -> Result<bool, TerminalError> {
        let mut workspace = self.terminal.workspace.clone();
        if !workspace.cycle_tab(delta) {
            return Ok(false);
        }
        self.persist_terminal_workspace(&workspace)?;
        self.terminal.workspace = workspace;
        if let Some(pane_id) = self.focused_terminal_pane_id() {
            self.mark_native_agent_panel_active(pane_id);
        }
        self.dirty = true;
        Ok(true)
    }

    pub fn focus_terminal_tab(&mut self, index: usize) -> Result<bool, TerminalError> {
        let mut workspace = self.terminal.workspace.clone();
        if !workspace.focus_tab(index) {
            return Ok(false);
        }
        self.persist_terminal_workspace(&workspace)?;
        self.terminal.workspace = workspace;
        if let Some(pane_id) = self.focused_terminal_pane_id() {
            self.mark_native_agent_panel_active(pane_id);
        }
        self.dirty = true;
        Ok(true)
    }

    fn runtime_terminal_definition(
        &self,
        mut definition: TerminalPaneDefinition,
    ) -> TerminalPaneDefinition {
        if definition.profile == TerminalProfile::OpenCode {
            definition.command = native_agent_command(&self.configured_opencode_command());
        }
        definition
    }
}

fn native_agent_command(command: &str) -> TerminalCommandDefinition {
    TerminalCommandDefinition {
        program: Some("/bin/sh".to_string()),
        args: vec!["-lc".to_string(), command.to_string()],
    }
}

fn validate_split_size(
    axis: TerminalSplitAxis,
    current: TerminalSize,
) -> Result<(), TerminalError> {
    let enough_room = match axis {
        TerminalSplitAxis::Columns => {
            current.columns >= MIN_SPLIT_CHILD_COLUMNS.saturating_mul(2).saturating_add(2)
        }
        TerminalSplitAxis::Rows => {
            current.rows >= MIN_SPLIT_CHILD_LINES.saturating_mul(2).saturating_add(2)
        }
    };
    if enough_room {
        Ok(())
    } else {
        Err(TerminalError::new(match axis {
            TerminalSplitAxis::Columns => format!(
                "pane is too narrow to split (need at least {} columns)",
                MIN_SPLIT_CHILD_COLUMNS * 2 + 2
            ),
            TerminalSplitAxis::Rows => format!(
                "pane is too short to split (need at least {} lines)",
                MIN_SPLIT_CHILD_LINES * 2 + 2
            ),
        }))
    }
}
