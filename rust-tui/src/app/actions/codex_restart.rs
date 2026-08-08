mod command {
    pub(super) fn build_codex_restart_command(
        agent_cmd: &str,
        cwd: &str,
        session_id: Option<&str>,
    ) -> String {
        let agent_cmd = agent_cmd.trim();
        let agent_cmd = if agent_cmd.is_empty() {
            "codex"
        } else {
            agent_cmd
        };
        let session_part = session_id
            .filter(|id| !id.trim().is_empty())
            .map(|id| shell_single_quote(id.trim()))
            .unwrap_or_else(|| "--last".to_string());

        format!(
            "exec {} -C {} resume {}",
            crate::codex_runtime::with_pad_codex_runtime(agent_cmd),
            shell_single_quote(cwd),
            session_part
        )
    }

    fn shell_single_quote(value: &str) -> String {
        crate::codex_runtime::shell_single_quote(value)
    }
}
mod messages {
    use super::super::helpers::{is_cjk_locale, localized};
    use crate::i18n::Locale;

    pub(super) fn restart_started_title(locale: Locale) -> &'static str {
        localized(locale, "Codex 重启中", "Codex Restarting")
    }

    pub(super) fn restart_failed_title(locale: Locale) -> &'static str {
        localized(locale, "Codex 重启失败", "Codex Restart Failed")
    }

    pub(super) fn restart_started_body(locale: Locale, session_id: Option<&str>) -> String {
        let session = session_id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or("--last");
        if is_cjk_locale(locale) {
            format!("恢复会话 {session}")
        } else {
            format!("Resuming {session}")
        }
    }
}
mod preflight {
    use super::super::helpers::localized;
    use crate::i18n::Locale;
    use crate::model::AgentType;

    pub(super) fn codex_restart_preflight_message(
        panel: &crate::model::AgentPanel,
        locale: Locale,
    ) -> Option<&'static str> {
        if panel.agent_type != AgentType::Codex {
            Some(codex_only_message(locale))
        } else {
            None
        }
    }

    pub(super) fn no_panel_message(locale: Locale) -> &'static str {
        localized(locale, "没有选中的面板", "No selected panel")
    }

    fn codex_only_message(locale: Locale) -> &'static str {
        localized(
            locale,
            "只支持 Codex 面板",
            "Only Codex panels can be restarted",
        )
    }
}

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
        let cwd = std::path::PathBuf::from(&panel.working_dir);
        let directory = cwd
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("workspace");
        let label = format!("Codex · {directory}");
        let size = self
            .focused_terminal_pane()
            .and_then(|pane| pane.size())
            .unwrap_or_else(|| crate::terminal_runtime::TerminalSize::new(80, 24));

        match self.launch_native_agent_terminal_at(
            &label,
            &command,
            crate::model::AgentType::Codex,
            cwd,
            size,
        ) {
            Ok(_) => {
                if App::is_native_agent_terminal_id(&panel.pane_id) {
                    if let Err(error) = self.close_native_agent_terminal(&panel.pane_id) {
                        self.show_action_toast(
                            restart_failed_title(self.locale),
                            &format!(
                                "new Codex pane started, but the previous pane could not close: {error}"
                            ),
                        );
                        return true;
                    }
                }
                self.focus_terminal();
                self.show_action_toast(
                    restart_started_title(self.locale),
                    &restart_started_body(self.locale, panel.agent_session_id.as_deref()),
                );
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
