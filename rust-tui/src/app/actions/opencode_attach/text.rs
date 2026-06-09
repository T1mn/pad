use super::super::helpers::localized;
use crate::i18n::Locale;

pub(super) fn attach_saved_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode 已 attach", "OpenCode Attached")
}

pub(super) fn attach_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode attach 失败", "OpenCode Attach Failed")
}
