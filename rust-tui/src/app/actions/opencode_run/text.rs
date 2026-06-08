use crate::i18n::Locale;

pub(super) fn run_started_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode run 已启动", "OpenCode Run Started")
}

pub(super) fn run_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode run 失败", "OpenCode Run Failed")
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
