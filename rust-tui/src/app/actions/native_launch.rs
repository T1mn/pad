use std::path::PathBuf;

use crate::app::TerminalPaneId;
use crate::model::AgentType;
use crate::terminal_runtime::{TerminalError, TerminalSize};

use super::{App, Mode};

impl App {
    pub(crate) fn configured_opencode_command(&self) -> String {
        super::opencode_cli::opencode_command(&self.config)
    }

    pub(in crate::app::actions) fn launch_native_agent_action(
        &mut self,
        label: &str,
        command: &str,
        agent_type: AgentType,
        cwd: PathBuf,
    ) -> Result<TerminalPaneId, TerminalError> {
        let size = self
            .focused_terminal_pane()
            .and_then(|pane| pane.size())
            .unwrap_or_else(|| TerminalSize::new(80, 24));
        self.sidebar.show_tree = false;
        self.mode = Mode::Normal;
        let pane_id =
            self.launch_native_agent_terminal_at(label, command, agent_type, cwd, size)?;
        let _ = self.focus_terminal();
        Ok(pane_id)
    }
}
