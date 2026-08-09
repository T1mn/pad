pub(crate) mod codex_restart;
mod helpers;
mod native_launch {
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
}
pub(crate) mod notification_inbox;
mod opencode_attach {
    mod command {
        pub(in crate::app::actions) fn attach_command(url: &str, command: &str) -> String {
            super::super::opencode_cli::command_with_args(command, ["attach", url])
        }
    }
    mod text {
        use super::super::helpers::localized;
        use crate::i18n::Locale;

        pub(super) fn attach_saved_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode 已 attach", "OpenCode Attached")
        }

        pub(super) fn attach_failed_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode attach 失败", "OpenCode Attach Failed")
        }
    }
    mod url {
        use super::super::helpers::trim_wrapping_quotes;

        pub(in crate::app::actions) fn normalize_server_url(
            text: &str,
        ) -> Result<String, &'static str> {
            let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
            let Some(first) = lines.next() else {
                return Err("Clipboard is empty");
            };
            if lines.next().is_some() {
                return Err("Clipboard must contain one OpenCode server URL");
            }
            let url = trim_wrapping_quotes(first).trim_end_matches('/');
            if is_http_url(url) && !url.contains(char::is_whitespace) {
                Ok(url.to_string())
            } else {
                Err("Clipboard must contain an http(s) OpenCode server URL")
            }
        }

        fn is_http_url(value: &str) -> bool {
            let rest = value
                .strip_prefix("http://")
                .or_else(|| value.strip_prefix("https://"));
            rest.is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
        }
    }

    use super::*;
    use std::path::PathBuf;

    impl App {
        pub fn attach_opencode_from_clipboard(&mut self) -> bool {
            let url = match url_from_clipboard() {
                Ok(url) => url,
                Err(message) => {
                    self.show_action_toast(text::attach_failed_title(self.locale), &message);
                    return false;
                }
            };
            let cwd = self
                .selected_preview_thread()
                .map(|thread| PathBuf::from(thread.working_dir))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let command =
                command::attach_command(&url, &opencode_cli::opencode_command(&self.config));
            match self.launch_native_agent_action(
                "OpenCode Attach",
                &command,
                AgentType::OpenCode,
                cwd,
            ) {
                Ok(_) => {
                    self.show_action_toast(text::attach_saved_title(self.locale), &url);
                    true
                }
                Err(err) => {
                    self.show_action_toast(
                        text::attach_failed_title(self.locale),
                        &err.to_string(),
                    );
                    false
                }
            }
        }
    }

    fn url_from_clipboard() -> Result<String, String> {
        let text =
            crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
        url::normalize_server_url(&text).map_err(str::to_string)
    }

    #[cfg(test)]
    pub(in crate::app::actions) use command::attach_command;
    #[cfg(test)]
    pub(in crate::app::actions) use url::normalize_server_url;
}
mod opencode_cli {
    use crate::theme::Config;
    use std::io;
    use std::path::Path;
    use std::process::{Command, Output};

    pub(super) fn opencode_command(config: &Config) -> String {
        config
            .agents
            .iter()
            .find(|agent| agent.name == "opencode")
            .map(|agent| agent.cmd.trim().to_string())
            .filter(|cmd| !cmd.is_empty())
            .unwrap_or_else(default_opencode_command)
    }

    pub(in crate::app::actions) fn command_with_args<'a>(
        command: &str,
        args: impl IntoIterator<Item = &'a str>,
    ) -> String {
        let mut command_line = command.trim().to_string();
        for arg in args {
            command_line.push(' ');
            command_line.push_str(&crate::shell_quote::single_quote(arg));
        }
        command_line
    }

    pub(in crate::app::actions) fn run_with_args(
        command: &str,
        args: &[&str],
        cwd: Option<&Path>,
    ) -> io::Result<Output> {
        let command_line = command_with_args(command, args.iter().copied());
        let mut process = Command::new("/bin/sh");
        process.args(["-lc", &command_line]);
        if let Some(cwd) = cwd {
            process.current_dir(cwd);
        }
        process.output()
    }

    pub(super) fn safe_filename(value: &str) -> String {
        let mut out = String::with_capacity(value.len().min(96));
        let mut sanitized_len = 0usize;
        let mut last_was_underscore = false;
        for ch in value.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                if sanitized_len == 0 && ch == '_' {
                    continue;
                }
                if out.len() < 96 {
                    out.push(ch);
                }
                sanitized_len += 1;
                last_was_underscore = ch == '_';
            } else if sanitized_len > 0 && !last_was_underscore {
                if out.len() < 96 {
                    out.push('_');
                }
                sanitized_len += 1;
                last_was_underscore = true;
            }
        }
        if sanitized_len <= 96 {
            while out.ends_with('_') {
                out.pop();
            }
        }
        if out.is_empty() {
            "session".to_string()
        } else {
            out
        }
    }

    fn default_opencode_command() -> String {
        let home_bin = crate::paths::pad_home_dir()
            .parent()
            .map(|home| home.join(".opencode").join("bin").join("opencode"));
        if let Some(path) = home_bin.filter(|path| path.exists()) {
            crate::shell_quote::single_quote(&path.to_string_lossy())
        } else {
            "opencode".to_string()
        }
    }
}
pub(crate) mod opencode_diagnostics;
mod opencode_export;
mod opencode_github {
    mod command {
        pub(in crate::app::actions) fn github_install_command(command: &str) -> String {
            super::super::opencode_cli::command_with_args(command, ["github", "install"])
        }
    }
    mod text {
        use super::super::helpers::localized;
        use crate::i18n::Locale;

        pub(super) fn github_started_title(locale: Locale) -> &'static str {
            localized(
                locale,
                "OpenCode GitHub install 已启动",
                "OpenCode GitHub Install Started",
            )
        }

        pub(super) fn github_failed_title(locale: Locale) -> &'static str {
            localized(
                locale,
                "OpenCode GitHub install 失败",
                "OpenCode GitHub Install Failed",
            )
        }
    }

    use super::*;
    use std::path::PathBuf;

    impl App {
        pub fn install_opencode_github_agent(&mut self) -> bool {
            let cwd = self
                .selected_preview_thread()
                .map(|thread| PathBuf::from(thread.working_dir))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let command =
                command::github_install_command(&opencode_cli::opencode_command(&self.config));
            match self.launch_native_agent_action(
                "OpenCode GitHub Install",
                &command,
                AgentType::OpenCode,
                cwd.clone(),
            ) {
                Ok(_) => {
                    self.show_action_toast(
                        text::github_started_title(self.locale),
                        &cwd.display().to_string(),
                    );
                    true
                }
                Err(err) => {
                    self.show_action_toast(
                        text::github_failed_title(self.locale),
                        &err.to_string(),
                    );
                    false
                }
            }
        }
    }

    #[cfg(test)]
    pub(in crate::app::actions) use command::github_install_command;
}
mod opencode_import;
mod opencode_plugin {
    mod command {
        pub(in crate::app::actions) fn plugin_command(module: &str, command: &str) -> String {
            super::super::opencode_cli::command_with_args(command, ["plugin", module])
        }
    }
    mod module {
        use super::super::helpers::trim_wrapping_quotes;

        pub(in crate::app::actions) fn normalize_plugin_module(
            text: &str,
        ) -> Result<String, &'static str> {
            let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
            let Some(first) = lines.next() else {
                return Err("Clipboard is empty");
            };
            if lines.next().is_some() {
                return Err("Clipboard must contain one npm module name");
            }
            let module = trim_wrapping_quotes(first);
            if is_safe_module_name(module) {
                Ok(module.to_string())
            } else {
                Err("Clipboard must contain an npm module name, not CLI flags or whitespace")
            }
        }

        fn is_safe_module_name(value: &str) -> bool {
            !value.is_empty()
                && !value.starts_with('-')
                && !value.contains(char::is_whitespace)
                && value.chars().all(|ch| {
                    ch.is_ascii_alphanumeric() || matches!(ch, '@' | '/' | '-' | '_' | '.' | '~')
                })
        }
    }
    mod text {
        use super::super::helpers::localized;
        use crate::i18n::Locale;

        pub(super) fn plugin_started_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode plugin 已启动", "OpenCode Plugin Started")
        }

        pub(super) fn plugin_failed_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode plugin 失败", "OpenCode Plugin Failed")
        }
    }

    use super::*;
    use std::path::PathBuf;

    impl App {
        pub fn install_opencode_plugin_from_clipboard(&mut self) -> bool {
            let module = match module_from_clipboard() {
                Ok(module) => module,
                Err(message) => {
                    self.show_action_toast(text::plugin_failed_title(self.locale), &message);
                    return false;
                }
            };

            let cwd = self
                .selected_preview_thread()
                .map(|thread| PathBuf::from(thread.working_dir))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let command =
                command::plugin_command(&module, &opencode_cli::opencode_command(&self.config));
            match self.launch_native_agent_action(
                "OpenCode Plugin Install",
                &command,
                AgentType::OpenCode,
                cwd,
            ) {
                Ok(_) => {
                    self.show_action_toast(text::plugin_started_title(self.locale), &module);
                    true
                }
                Err(err) => {
                    self.show_action_toast(
                        text::plugin_failed_title(self.locale),
                        &err.to_string(),
                    );
                    false
                }
            }
        }
    }

    fn module_from_clipboard() -> Result<String, String> {
        let text =
            crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
        module::normalize_plugin_module(&text).map_err(str::to_string)
    }

    #[cfg(test)]
    pub(in crate::app::actions) use command::plugin_command;
    #[cfg(test)]
    pub(in crate::app::actions) use module::normalize_plugin_module;
}
mod opencode_pr {
    mod command {
        pub(in crate::app::actions) fn pr_command(pr_number: &str, command: &str) -> String {
            super::super::opencode_cli::command_with_args(command, ["pr", pr_number])
        }
    }
    mod parse {
        use super::super::helpers::trim_wrapping_quotes;

        pub(in crate::app::actions) fn normalize_pr_number(
            text: &str,
        ) -> Result<String, &'static str> {
            let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
            let Some(first) = lines.next() else {
                return Err("Clipboard is empty");
            };
            if lines.next().is_some() {
                return Err("Clipboard must contain one PR number or URL");
            }
            let value = trim_wrapping_quotes(first);
            let candidate = value
                .strip_prefix('#')
                .or_else(|| number_after_pull_segment(value))
                .unwrap_or(value);
            if is_positive_integer(candidate) {
                Ok(candidate.to_string())
            } else {
                Err("Clipboard must contain a GitHub PR number or /pull/<number> URL")
            }
        }

        fn number_after_pull_segment(value: &str) -> Option<&str> {
            let marker = "/pull/";
            let start = value.find(marker)? + marker.len();
            let tail = &value[start..];
            let len = tail
                .char_indices()
                .find_map(|(idx, ch)| (!ch.is_ascii_digit()).then_some(idx))
                .unwrap_or(tail.len());
            (len > 0).then_some(&tail[..len])
        }

        fn is_positive_integer(value: &str) -> bool {
            !value.is_empty() && value != "0" && value.chars().all(|ch| ch.is_ascii_digit())
        }
    }
    mod text {
        use super::super::helpers::localized;
        use crate::i18n::Locale;

        pub(super) fn pr_started_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode PR 已启动", "OpenCode PR Started")
        }

        pub(super) fn pr_failed_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode PR 失败", "OpenCode PR Failed")
        }
    }

    use super::*;
    use std::path::PathBuf;

    impl App {
        pub fn open_opencode_pr_from_clipboard(&mut self) -> bool {
            let pr_number = match pr_number_from_clipboard() {
                Ok(number) => number,
                Err(message) => {
                    self.show_action_toast(text::pr_failed_title(self.locale), &message);
                    return false;
                }
            };

            let cwd = self
                .selected_preview_thread()
                .map(|thread| PathBuf::from(thread.working_dir))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let command =
                command::pr_command(&pr_number, &opencode_cli::opencode_command(&self.config));
            match self.launch_native_agent_action("OpenCode PR", &command, AgentType::OpenCode, cwd)
            {
                Ok(_) => {
                    self.show_action_toast(
                        text::pr_started_title(self.locale),
                        &format!("PR #{pr_number}"),
                    );
                    true
                }
                Err(err) => {
                    self.show_action_toast(text::pr_failed_title(self.locale), &err.to_string());
                    false
                }
            }
        }
    }

    fn pr_number_from_clipboard() -> Result<String, String> {
        let text =
            crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
        parse::normalize_pr_number(&text).map_err(str::to_string)
    }

    #[cfg(test)]
    pub(in crate::app::actions) use command::pr_command;
    #[cfg(test)]
    pub(in crate::app::actions) use parse::normalize_pr_number;
}
mod opencode_run;
mod opencode_serve {
    mod command {
        pub(in crate::app::actions) fn serve_command(command: &str) -> String {
            super::super::opencode_cli::command_with_args(
                command,
                ["serve", "--hostname", "127.0.0.1", "--port", "0"],
            )
        }
    }
    mod text {
        use super::super::helpers::localized;
        use crate::i18n::Locale;

        pub(super) fn serve_started_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode serve 已启动", "OpenCode Serve Started")
        }

        pub(super) fn serve_failed_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode serve 失败", "OpenCode Serve Failed")
        }
    }

    use super::*;
    use std::path::PathBuf;

    impl App {
        pub fn serve_opencode_for_selected_thread(&mut self) -> bool {
            let cwd = self
                .selected_preview_thread()
                .map(|thread| PathBuf::from(thread.working_dir))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let command = command::serve_command(&opencode_cli::opencode_command(&self.config));
            match self.launch_native_agent_action(
                "OpenCode Serve",
                &command,
                AgentType::OpenCode,
                cwd.clone(),
            ) {
                Ok(_) => {
                    self.show_action_toast(
                        text::serve_started_title(self.locale),
                        &cwd.display().to_string(),
                    );
                    true
                }
                Err(err) => {
                    self.show_action_toast(text::serve_failed_title(self.locale), &err.to_string());
                    false
                }
            }
        }
    }

    #[cfg(test)]
    pub(in crate::app::actions) use command::serve_command;
}
pub(crate) mod opencode_stats;
mod opencode_web {
    mod command {
        pub(in crate::app::actions) fn web_command(command: &str) -> String {
            super::super::opencode_cli::command_with_args(command, ["web"])
        }
    }
    mod text {
        use super::super::helpers::localized;
        use crate::i18n::Locale;

        pub(super) fn web_opened_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode Web 已打开", "OpenCode Web Opened")
        }

        pub(super) fn web_failed_title(locale: Locale) -> &'static str {
            localized(locale, "OpenCode Web 失败", "OpenCode Web Failed")
        }
    }

    use super::*;
    use std::path::PathBuf;

    impl App {
        pub fn open_opencode_web_for_selected_thread(&mut self) -> bool {
            let cwd = self
                .selected_preview_thread()
                .map(|thread| PathBuf::from(thread.working_dir))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let command = command::web_command(&opencode_cli::opencode_command(&self.config));
            match self.launch_native_agent_action(
                "OpenCode Web",
                &command,
                AgentType::OpenCode,
                cwd.clone(),
            ) {
                Ok(_) => {
                    self.show_action_toast(
                        text::web_opened_title(self.locale),
                        &cwd.display().to_string(),
                    );
                    true
                }
                Err(err) => {
                    self.show_action_toast(text::web_failed_title(self.locale), &err.to_string());
                    false
                }
            }
        }
    }

    #[cfg(test)]
    pub(in crate::app::actions) use command::web_command;
}
mod panel_width {
    use super::App;

    const AGENT_PANEL_WIDTH_STEP: u16 = 6;
    const MAX_AGENT_PANEL_WIDTH: u16 = 90;

    impl App {
        pub fn widen_agent_panel_width(&mut self, current_width: u16) {
            let next = current_width
                .saturating_add(AGENT_PANEL_WIDTH_STEP)
                .min(MAX_AGENT_PANEL_WIDTH);
            self.config.display.agent_panel_width = Some(next);
            self.sidebar.preferred_panel_width_cache = None;
            if self.save_config() {
                self.show_action_toast(
                    panel_width_toast_title(self.locale),
                    &panel_width_toast_body(self.locale, next),
                );
            }
            self.dirty = true;
        }
    }

    fn panel_width_toast_title(locale: crate::i18n::Locale) -> &'static str {
        match locale {
            crate::i18n::Locale::ZhCN => "左侧宽度已保存",
            crate::i18n::Locale::ZhTW => "左側寬度已儲存",
            _ => "Sidebar width saved",
        }
    }

    fn panel_width_toast_body(locale: crate::i18n::Locale, width: u16) -> String {
        match locale {
            crate::i18n::Locale::ZhCN => format!("Agent 列表宽度：{width}"),
            crate::i18n::Locale::ZhTW => format!("Agent 列表寬度：{width}"),
            _ => format!("Agent list width: {width}"),
        }
    }
}
pub(crate) mod relay_reload;
mod settings;
pub(crate) mod thread_actions;
mod thread_meta_edit;
mod thread_panel_delete {
    use super::helpers::delete_failed_title;
    use super::*;

    impl App {
        pub fn delete_panel(&mut self, panel: &crate::model::AgentPanel) {
            self.sidebar.pending_sidebar_selection_index = self.table_state.selected();
            log_debug!(
                "delete_panel: pane_id={} runtime=native agent_type={:?}",
                panel.pane_id,
                panel.agent_type,
            );

            if App::is_native_agent_terminal_id(&panel.pane_id) {
                match self.close_native_agent_terminal(&panel.pane_id) {
                    Ok(true) => {
                        self.invalidate_preview();
                        self.focus_panel();
                    }
                    Ok(false) => self.show_action_toast(
                        delete_failed_title(self.locale),
                        "native terminal pane no longer exists",
                    ),
                    Err(error) => {
                        self.show_action_toast(delete_failed_title(self.locale), &error.to_string())
                    }
                }
                return;
            }

            self.apply_deleted_panel_locally(&panel.pane_id);
            self.show_action_toast(
                delete_failed_title(self.locale),
                "removed stale non-native panel entry",
            );
        }

        pub(crate) fn apply_deleted_panel_locally(&mut self, pane_id: &str) {
            let original_len = self.panels.len();
            self.panels.retain(|panel| panel.pane_id != pane_id);
            if self.panels.len() == original_len {
                return;
            }

            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
            if self.selected_panel().is_none() {
                self.focus_panel();
            }
            self.invalidate_preview();
            self.dirty = true;
        }

        pub fn refresh_panels(&mut self) {
            self.last_refresh = std::time::Instant::now();
            self.invalidate_sidebar_cache();
            self.invalidate_preview();
            self.dirty = true;
        }
    }
}
mod tree;

use super::state::{Mode, SettingsDetailKind, SettingsFocus};
use super::{App, ThreadActionKind, ThreadMetaEditKind};
use crate::i18n::Locale;
use crate::log_debug;
use crate::model::AgentType;
use crate::sidebar::{SidebarItem, SidebarThread};

pub(crate) use helpers::settings_item_search_blob;

#[cfg(test)]
#[path = "actions/opencode_tests.rs"]
pub(crate) mod opencode_tests;

#[cfg(test)]
pub(crate) mod tests;
