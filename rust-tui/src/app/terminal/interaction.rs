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
        let TerminalInteractionState::Rename { buffer, .. } = &mut self.terminal.interaction else {
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
        let TerminalInteractionState::Rename { buffer, .. } = &mut self.terminal.interaction else {
            return false;
        };
        let changed = buffer.pop().is_some();
        self.dirty |= changed;
        changed
    }

    pub fn clear_terminal_rename(&mut self) -> bool {
        let TerminalInteractionState::Rename { buffer, .. } = &mut self.terminal.interaction else {
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
                self.terminal.interaction = TerminalInteractionState::Rename { pane_id, buffer };
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
