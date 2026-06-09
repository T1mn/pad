use super::super::helpers::localized;
use crate::i18n::Locale;

pub(super) fn plugin_started_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode plugin 已启动", "OpenCode Plugin Started")
}

pub(super) fn plugin_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode plugin 失败", "OpenCode Plugin Failed")
}
