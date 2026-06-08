use crate::i18n::Locale;

pub(super) fn plugin_started_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode plugin 已启动", "OpenCode Plugin Started")
}

pub(super) fn plugin_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode plugin 失败", "OpenCode Plugin Failed")
}

fn localized(locale: Locale, zh: &'static str, en: &'static str) -> &'static str {
    if is_cjk_locale(locale) {
        zh
    } else {
        en
    }
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}
