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
