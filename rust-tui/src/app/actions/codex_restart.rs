mod command;
mod messages;
mod preflight;

#[cfg(test)]
mod tests;

use super::*;
use command::build_codex_restart_command;
use messages::{restart_failed_title, restart_started_body, restart_started_title};
use preflight::{codex_restart_preflight_message, no_panel_message};

impl App {
    pub fn restart_selected_codex_panel(&mut self) -> bool {
        let Some(panel) = self.selected_panel().cloned() else {
            self.show_action_toast(
                restart_failed_title(self.locale),
                no_panel_message(self.locale),
            );
            return false;
        };

        if let Some(message) = codex_restart_preflight_message(&panel, self.locale) {
            self.show_action_toast(restart_failed_title(self.locale), message);
            return false;
        }

        if let Err(err) = crate::paths::write_codex_selected_prompt_file(
            self.config.codex.jailbreak_prompt_file,
            self.config.codex.index_prompt_file,
        ) {
            self.show_action_toast(restart_failed_title(self.locale), &err.to_string());
            return false;
        }
        if let Err(err) = crate::paths::ensure_pad_codex_home_layout() {
            self.show_action_toast(restart_failed_title(self.locale), &err.to_string());
            return false;
        }
        crate::relay::apply_runtime_overlays(
            &self.config.agents,
            &self.config.agent_permissions,
            &self.config.codex,
        );
        if let Err(err) = crate::paths::ensure_pad_codex_wrapper()
            .and_then(|_| crate::codex_runtime::ensure_pad_codex_auth_ready())
        {
            self.show_action_toast(restart_failed_title(self.locale), &err.to_string());
            return false;
        }

        let agent_cmd = self.codex_agent_command();
        let command = build_codex_restart_command(
            agent_cmd,
            &panel.working_dir,
            panel.agent_session_id.as_deref(),
        );

        match crate::tmux_dispatch::respawn_pane_shell(&panel.pane_id, &panel.working_dir, &command)
        {
            Ok(()) => {
                self.show_action_toast(
                    restart_started_title(self.locale),
                    &restart_started_body(self.locale, panel.agent_session_id.as_deref()),
                );
                self.refresh_panels();
                true
            }
            Err(err) => {
                self.show_action_toast(restart_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }

    fn codex_agent_command(&self) -> &str {
        self.config
            .agents
            .iter()
            .find(|agent| agent.name == "codex")
            .map(|agent| agent.cmd.trim())
            .filter(|cmd| !cmd.is_empty())
            .unwrap_or("codex")
    }
}
