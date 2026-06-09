use super::super::helpers::localized;
use crate::i18n::Locale;

pub(super) fn pr_started_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode PR 已启动", "OpenCode PR Started")
}

pub(super) fn pr_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode PR 失败", "OpenCode PR Failed")
}
