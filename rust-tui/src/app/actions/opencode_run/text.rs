use super::super::helpers::localized;
use crate::i18n::Locale;

pub(super) fn run_started_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode run 已启动", "OpenCode Run Started")
}

pub(super) fn run_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode run 失败", "OpenCode Run Failed")
}
