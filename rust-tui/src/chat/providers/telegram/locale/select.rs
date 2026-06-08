use crate::theme::Config;

pub(in crate::chat::providers::telegram) fn telegram_locale(
    config: &Config,
) -> crate::i18n::Locale {
    crate::i18n::Locale::from_str(&config.language)
}

pub(in crate::chat::providers::telegram) fn locale_prefers_chinese(
    locale: crate::i18n::Locale,
) -> bool {
    matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    )
}
