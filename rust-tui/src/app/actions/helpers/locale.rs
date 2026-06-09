use crate::i18n::Locale;

pub(in crate::app::actions) fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

pub(in crate::app::actions) fn localized(
    locale: Locale,
    zh: &'static str,
    en: &'static str,
) -> &'static str {
    if is_cjk_locale(locale) {
        zh
    } else {
        en
    }
}
