use crate::i18n::Locale;

pub(super) fn diagnostics_saved_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 诊断已导出"
    } else {
        "OpenCode Diagnostics Exported"
    }
}

pub(super) fn diagnostics_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 诊断失败"
    } else {
        "OpenCode Diagnostics Failed"
    }
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}
