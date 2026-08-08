use std::sync::Arc;

use crate::terminal_runtime::{
    PaneFrame, TerminalError, TerminalMode, TerminalScroll, TerminalSize, TransportExit,
};

use super::{App, TerminalInteractionState, TerminalPaneId, TerminalPaneLifecycle};

#[cfg(test)]
thread_local! {
    static FAIL_NEXT_WORKSPACE_SAVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(super) fn fail_next_workspace_save() {
    FAIL_NEXT_WORKSPACE_SAVE.with(|flag| flag.set(true));
}

impl App {
    pub fn terminal_mode(&self) -> TerminalMode {
        self.terminal.mode()
    }

    pub fn terminal_frame(&self) -> Option<&Arc<PaneFrame>> {
        self.terminal.frame()
    }

    pub fn terminal_error(&self) -> Option<&str> {
        self.terminal.error()
    }

    pub fn terminal_exit(&self) -> Option<TransportExit> {
        self.terminal.exit()
    }

    /// Queues keyboard or paste input for the focused pane. The pane viewport
    /// is returned to the live bottom immediately before the bytes.
    pub fn send_terminal_input(&mut self, bytes: Vec<u8>) -> Result<(), TerminalError> {
        let pane_id = self.require_focused_terminal_pane()?;
        let result = self.terminal.queue_input(pane_id, bytes, true);
        self.dirty = true;
        result
    }

    /// Sends child mouse-reporting bytes without changing PAD scrollback.
    pub fn send_terminal_mouse_input(&mut self, bytes: Vec<u8>) -> Result<(), TerminalError> {
        let pane_id = self.require_focused_terminal_pane()?;
        self.terminal.queue_input(pane_id, bytes, false)
    }

    pub fn send_native_pane_input(
        &mut self,
        live_pane_id: &str,
        bytes: Vec<u8>,
    ) -> Result<(), TerminalError> {
        let pane_id = super::agent_registry::parse_native_agent_pane_id(live_pane_id)
            .ok_or_else(|| TerminalError::new("unknown native terminal pane id"))?;
        let lifecycle = self
            .terminal
            .pane(pane_id)
            .ok_or_else(|| TerminalError::new("native terminal pane no longer exists"))?
            .lifecycle();
        if !matches!(
            lifecycle,
            TerminalPaneLifecycle::Opening | TerminalPaneLifecycle::Running
        ) {
            return Err(TerminalError::new(format!(
                "native terminal pane is not running ({lifecycle:?})"
            )));
        }
        let result = self.terminal.queue_input(pane_id, bytes, true);
        self.dirty = true;
        result
    }

    pub fn native_pane_text(&self, live_pane_id: &str) -> Option<String> {
        let pane_id = super::agent_registry::parse_native_agent_pane_id(live_pane_id)?;
        let frame = self.terminal.pane(pane_id)?.frame()?;
        let mut output = String::new();
        for row in 0..frame.terminal.size.rows {
            let text = frame.terminal.row_text(row)?;
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&text);
        }
        Some(output.trim_end_matches('\n').to_string())
    }

    pub fn scroll_terminal(&mut self, scroll: TerminalScroll) -> Result<(), TerminalError> {
        let pane_id = self.require_focused_terminal_pane()?;
        let result = self.terminal.queue_scroll(pane_id, scroll);
        self.dirty = true;
        result
    }

    pub fn resize_native_terminals(&mut self, sizes: &[(TerminalPaneId, TerminalSize)]) {
        for &(pane_id, size) in sizes {
            self.terminal.queue_resize(pane_id, size);
        }
        self.terminal.flush_commands();
    }

    pub fn poll_native_terminal(&mut self) {
        self.dirty |= self.terminal.poll_frames();
    }

    pub fn shutdown_native_terminal(&mut self) -> Result<(), TerminalError> {
        let controller = self.terminal.controller.take();
        self.terminal.frames = None;
        self.terminal.panes.clear();
        self.terminal.flush_order.clear();
        self.terminal.flush_cursor = 0;
        self.terminal.interaction = TerminalInteractionState::Direct;
        if self.remove_all_native_agent_panels() {
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
            self.dirty = true;
        }
        if let Some(controller) = controller {
            controller.shutdown()?;
        }
        Ok(())
    }

    pub(super) fn ensure_terminal_controller(&self) -> Result<(), TerminalError> {
        if self.terminal.controller.is_some() {
            Ok(())
        } else {
            Err(TerminalError::new(
                "native terminal controller has not been started",
            ))
        }
    }

    fn require_focused_terminal_pane(&self) -> Result<TerminalPaneId, TerminalError> {
        self.focused_terminal_pane_id()
            .ok_or_else(|| TerminalError::new("native terminal workspace has no focused pane"))
    }

    pub(super) fn persist_terminal_workspace(
        &self,
        workspace: &super::TerminalWorkspace,
    ) -> Result<(), TerminalError> {
        #[cfg(test)]
        {
            workspace.validate().map_err(TerminalError::new)?;
            if FAIL_NEXT_WORKSPACE_SAVE.with(|flag| flag.replace(false)) {
                Err(TerminalError::new(
                    "injected terminal workspace save failure",
                ))
            } else {
                Ok(())
            }
        }
        #[cfg(not(test))]
        {
            crate::terminal_workspace::save(workspace).map_err(|error| {
                crate::log_debug!("terminal workspace save failed: {}", error);
                TerminalError::new(format!("terminal workspace save failed: {error}"))
            })
        }
    }
}
