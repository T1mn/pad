use super::super::helpers::localized;
use crate::i18n::Locale;

pub(super) fn web_opened_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode Web 已打开", "OpenCode Web Opened")
}

pub(super) fn web_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode Web 失败", "OpenCode Web Failed")
}
