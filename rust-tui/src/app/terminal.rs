mod agent_registry;
mod controller_io;
mod interaction;
mod model;
mod runtime_io;

#[cfg(test)]
mod tests;

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
        for definition in &definitions {
            self.terminal.install_pane_runtime(definition, initial_size);
        }
        self.terminal.flush_commands();
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
        let pane_id = self
            .terminal
            .workspace
            .add_tab(profile, cwd)
            .ok_or_else(|| {
                TerminalError::new(format!(
                    "terminal workspace is limited to {} tabs and {} panes",
                    model::MAX_TERMINAL_TABS,
                    model::MAX_TERMINAL_PANES
                ))
            })?;
        let definition = self
            .terminal
            .workspace
            .pane(pane_id)
            .expect("new terminal tab has a pane definition")
            .clone();
        self.terminal.install_pane_runtime(&definition, size);
        self.terminal.flush_commands();
        self.mark_native_agent_panel_active(pane_id);
        self.dirty = true;
        self.persist_terminal_workspace();
        Ok(pane_id)
    }

    /// Opens a persistent shell tab and starts a configured agent command in
    /// that shell. The command is runtime-only: workspace restore keeps the
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
        let pane_id = self.create_terminal_tab_at(TerminalProfile::Shell, cwd.clone(), size)?;
        self.rename_terminal_pane(pane_id, &label)?;
        let mut input = command.as_bytes().to_vec();
        input.push(b'\r');
        if let Err(error) = self.terminal.queue_input(pane_id, input, true) {
            self.close_terminal_pane(pane_id);
            return Err(error);
        }
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
        let pane_id = self
            .terminal
            .workspace
            .split_focused(axis, profile, cwd)
            .ok_or_else(|| TerminalError::new("terminal workspace has no focused pane to split"))?;
        let definition = self
            .terminal
            .workspace
            .pane(pane_id)
            .expect("new terminal split has a pane definition")
            .clone();
        self.terminal.install_pane_runtime(&definition, size);
        self.terminal.flush_commands();
        self.mark_native_agent_panel_active(pane_id);
        self.dirty = true;
        self.persist_terminal_workspace();
        Ok(pane_id)
    }

    pub fn close_terminal_pane(&mut self, pane_id: TerminalPaneId) -> bool {
        if !self.terminal.workspace.close_pane(pane_id) {
            return false;
        }
        self.terminal.queue_close(pane_id);
        self.remove_native_agent_panel(pane_id);
        if let Some(focused) = self.focused_terminal_pane_id() {
            self.mark_native_agent_panel_active(focused);
        }
        self.dirty = true;
        self.persist_terminal_workspace();
        true
    }

    pub fn close_focused_terminal(&mut self) -> bool {
        self.focused_terminal_pane_id()
            .is_some_and(|pane_id| self.close_terminal_pane(pane_id))
    }

    pub fn rename_terminal_pane(
        &mut self,
        pane_id: TerminalPaneId,
        label: &str,
    ) -> Result<bool, TerminalError> {
        let label = model::normalize_label(label).map_err(TerminalError::new)?;
        if !self.terminal.workspace.rename_pane(pane_id, label.clone()) {
            return Ok(false);
        }
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
        self.persist_terminal_workspace();
        Ok(true)
    }

    pub fn focus_terminal_pane(&mut self, pane_id: TerminalPaneId) -> bool {
        if !self.terminal.workspace.focus_pane(pane_id) {
            return false;
        }
        self.mark_native_agent_panel_active(pane_id);
        self.dirty = true;
        self.persist_terminal_workspace();
        true
    }

    pub fn cycle_terminal_pane(&mut self, delta: isize) -> bool {
        let changed = self.terminal.workspace.cycle_pane(delta);
        self.dirty |= changed;
        if changed {
            if let Some(pane_id) = self.focused_terminal_pane_id() {
                self.mark_native_agent_panel_active(pane_id);
            }
            self.persist_terminal_workspace();
        }
        changed
    }

    pub fn cycle_terminal_tab(&mut self, delta: isize) -> bool {
        let changed = self.terminal.workspace.cycle_tab(delta);
        self.dirty |= changed;
        if changed {
            if let Some(pane_id) = self.focused_terminal_pane_id() {
                self.mark_native_agent_panel_active(pane_id);
            }
            self.persist_terminal_workspace();
        }
        changed
    }

    pub fn focus_terminal_tab(&mut self, index: usize) -> bool {
        let changed = self.terminal.workspace.focus_tab(index);
        self.dirty |= changed;
        if changed {
            if let Some(pane_id) = self.focused_terminal_pane_id() {
                self.mark_native_agent_panel_active(pane_id);
            }
            self.persist_terminal_workspace();
        }
        changed
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
