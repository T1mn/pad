use super::*;

impl App {
    pub fn restart_selected_codex_panel(&mut self) -> bool {
        let Some(panel) = self.selected_panel().cloned() else {
            self.show_action_toast(
                restart_failed_title(self.locale),
                no_panel_message(self.locale),
            );
            return false;
        };

        if panel.agent_type != AgentType::Codex {
            self.show_action_toast(
                restart_failed_title(self.locale),
                codex_only_message(self.locale),
            );
            return false;
        }

        if panel.state != crate::model::AgentState::Idle {
            self.show_action_toast(
                restart_failed_title(self.locale),
                codex_busy_message(self.locale),
            );
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

fn build_codex_restart_command(agent_cmd: &str, cwd: &str, session_id: Option<&str>) -> String {
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
        crate::codex_runtime::with_codex_home(agent_cmd),
        shell_single_quote(cwd),
        session_part
    )
}

fn shell_single_quote(value: &str) -> String {
    crate::codex_runtime::shell_single_quote(value)
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

fn restart_started_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "Codex 重启中"
    } else {
        "Codex Restarting"
    }
}

fn restart_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "Codex 重启失败"
    } else {
        "Codex Restart Failed"
    }
}

fn restart_started_body(locale: Locale, session_id: Option<&str>) -> String {
    let session = session_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or("--last");
    if is_cjk_locale(locale) {
        format!("恢复会话 {session}")
    } else {
        format!("Resuming {session}")
    }
}

fn no_panel_message(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "没有选中的面板"
    } else {
        "No selected panel"
    }
}

fn codex_only_message(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "只支持 Codex 面板"
    } else {
        "Only Codex panels can be restarted"
    }
}

fn codex_busy_message(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "Codex 正在运行中，等空闲后再重启"
    } else {
        "Codex is busy; restart when idle"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_command_parts(command: &str, suffix: &str) {
        assert!(
            command.starts_with("exec env CODEX_HOME='"),
            "missing CODEX_HOME prefix: {command}"
        );
        assert!(
            command.ends_with(suffix),
            "unexpected command suffix: {command}"
        );
    }

    #[test]
    fn restart_command_resumes_specific_session() {
        assert_command_parts(
            &build_codex_restart_command("codex", "/tmp/project", Some("sid-1")),
            " codex -C '/tmp/project' resume 'sid-1'",
        );
    }

    #[test]
    fn restart_command_falls_back_to_last_session() {
        assert_command_parts(
            &build_codex_restart_command("codex", "/tmp/project", None),
            " codex -C '/tmp/project' resume --last",
        );
    }

    #[test]
    fn restart_command_quotes_shell_values() {
        assert_command_parts(
            &build_codex_restart_command("codex --profile work", "/tmp/a'b", Some("s'id")),
            r" codex --profile work -C '/tmp/a'\''b' resume 's'\''id'",
        );
    }
}
