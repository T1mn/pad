mod export;
mod path {
    use std::path::{Path, PathBuf};

    pub(in crate::app::actions) fn opencode_stats_path(
        project: &str,
        dir: &Path,
        timestamp: u64,
    ) -> PathBuf {
        let stem = super::super::opencode_cli::safe_filename(project)
            .trim_start_matches('_')
            .to_string();
        dir.join(format!("{}-{}.txt", stem, timestamp))
    }

    pub(super) fn current_unix_secs() -> u64 {
        crate::time::unix_now_secs()
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn stats_saved_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode stats 已导出", "OpenCode Stats Exported")
    }

    pub(super) fn stats_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode stats 失败", "OpenCode Stats Failed")
    }

    pub(super) fn no_thread_message(locale: Locale) -> &'static str {
        localized(locale, "没有选中的线程", "No selected thread")
    }

    pub(super) fn opencode_only_message(locale: Locale) -> &'static str {
        localized(locale, "只支持 OpenCode 会话", "Only OpenCode sessions")
    }
}

use super::*;

impl App {
    pub fn export_selected_opencode_stats(&mut self) -> bool {
        let Some(thread) = self.selected_preview_thread() else {
            self.show_action_toast(
                text::stats_failed_title(self.locale),
                text::no_thread_message(self.locale),
            );
            return false;
        };
        if thread.agent_type != AgentType::OpenCode {
            self.show_action_toast(
                text::stats_failed_title(self.locale),
                text::opencode_only_message(self.locale),
            );
            return false;
        }

        match export::export_opencode_stats(
            &thread.working_dir,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(path) => {
                self.show_action_toast(
                    text::stats_saved_title(self.locale),
                    &path.display().to_string(),
                );
                true
            }
            Err(err) => {
                self.show_action_toast(text::stats_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

#[cfg(test)]
pub(in crate::app::actions) use path::opencode_stats_path;

#[cfg(test)]
#[path = "opencode_stats_tests.rs"]
mod opencode_stats_tests;
