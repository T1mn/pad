use crate::i18n::Locale;

pub(super) fn attach_saved_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 已 attach"
    } else {
        "OpenCode Attached"
    }
}

pub(super) fn attach_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode attach 失败"
    } else {
        "OpenCode Attach Failed"
    }
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}
