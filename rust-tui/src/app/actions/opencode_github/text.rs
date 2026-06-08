use crate::i18n::Locale;

pub(super) fn github_started_title(locale: Locale) -> &'static str {
    localized(
        locale,
        "OpenCode GitHub install 已启动",
        "OpenCode GitHub Install Started",
    )
}

pub(super) fn github_failed_title(locale: Locale) -> &'static str {
    localized(
        locale,
        "OpenCode GitHub install 失败",
        "OpenCode GitHub Install Failed",
    )
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
