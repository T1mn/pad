use super::super::helpers::localized;
use crate::i18n::Locale;

pub(super) fn serve_started_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode serve 已启动", "OpenCode Serve Started")
}

pub(super) fn serve_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode serve 失败", "OpenCode Serve Failed")
}
