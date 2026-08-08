mod export;
mod mode {
    #[derive(Clone, Copy)]
    pub(in crate::app::actions) enum ExportMode {
        Raw,
        Sanitized,
    }
}
mod path {
    use super::mode::ExportMode;
    use std::path::{Path, PathBuf};

    pub(in crate::app::actions) fn opencode_export_path(
        session_id: &str,
        dir: &Path,
        mode: ExportMode,
    ) -> PathBuf {
        let suffix = match mode {
            ExportMode::Raw => "json",
            ExportMode::Sanitized => "sanitized.json",
        };
        dir.join(format!(
            "{}.{}",
            super::super::opencode_cli::safe_filename(session_id),
            suffix
        ))
    }
}
mod text {
    use super::super::helpers::{is_cjk_locale, localized};
    use super::mode::ExportMode;
    use crate::i18n::Locale;

    pub(super) fn export_saved_title(locale: Locale, mode: ExportMode) -> &'static str {
        match (is_cjk_locale(locale), mode) {
            (true, ExportMode::Raw) => "OpenCode 已导出",
            (true, ExportMode::Sanitized) => "OpenCode 已脱敏导出",
            (false, ExportMode::Raw) => "OpenCode Exported",
            (false, ExportMode::Sanitized) => "OpenCode Sanitized Exported",
        }
    }

    pub(super) fn export_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode 导出失败", "OpenCode Export Failed")
    }

    pub(super) fn no_thread_message(locale: Locale) -> &'static str {
        localized(locale, "没有选中的线程", "No selected thread")
    }

    pub(super) fn opencode_only_message(locale: Locale) -> &'static str {
        localized(locale, "只支持 OpenCode 会话", "Only OpenCode sessions")
    }

    pub(super) fn missing_session_message(locale: Locale) -> &'static str {
        localized(
            locale,
            "选中的 OpenCode 线程缺少 session id",
            "Missing OpenCode session id",
        )
    }
}

use super::{opencode_cli, App};
use crate::model::AgentType;
pub(in crate::app::actions) use mode::ExportMode;
use text::{
    export_failed_title, export_saved_title, missing_session_message, no_thread_message,
    opencode_only_message,
};

impl App {
    pub fn export_selected_opencode_thread(&mut self) -> bool {
        self.export_selected_opencode_thread_with_options(ExportMode::Raw)
    }

    pub fn export_sanitized_selected_opencode_thread(&mut self) -> bool {
        self.export_selected_opencode_thread_with_options(ExportMode::Sanitized)
    }

    fn export_selected_opencode_thread_with_options(&mut self, mode: ExportMode) -> bool {
        let Some(thread) = self.selected_preview_thread() else {
            self.show_action_toast(
                export_failed_title(self.locale),
                no_thread_message(self.locale),
            );
            return false;
        };
        if thread.agent_type != AgentType::OpenCode {
            self.show_action_toast(
                export_failed_title(self.locale),
                opencode_only_message(self.locale),
            );
            return false;
        }
        let Some(session_id) = thread
            .session_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
        else {
            self.show_action_toast(
                export_failed_title(self.locale),
                missing_session_message(self.locale),
            );
            return false;
        };

        match export::export_opencode_session(
            session_id,
            &opencode_cli::opencode_command(&self.config),
            mode,
        ) {
            Ok(path) => {
                self.show_action_toast(
                    export_saved_title(self.locale, mode),
                    &path.display().to_string(),
                );
                true
            }
            Err(err) => {
                self.show_action_toast(export_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

#[cfg(test)]
pub(in crate::app::actions) use path::opencode_export_path;
