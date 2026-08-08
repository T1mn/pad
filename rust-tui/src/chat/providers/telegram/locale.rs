mod format {
    use super::text::tg;

    pub(in crate::chat::providers::telegram) fn tg_fmt(
        locale: crate::i18n::Locale,
        key: &str,
        arg: impl std::fmt::Display,
    ) -> String {
        tg(locale, key).replacen("{}", &arg.to_string(), 1)
    }

    pub(in crate::chat::providers::telegram) fn tg_fmt2(
        locale: crate::i18n::Locale,
        key: &str,
        arg1: impl std::fmt::Display,
        arg2: impl std::fmt::Display,
    ) -> String {
        tg(locale, key)
            .replacen("{}", &arg1.to_string(), 1)
            .replacen("{}", &arg2.to_string(), 1)
    }

    pub(in crate::chat::providers::telegram) fn tg_fmt3(
        locale: crate::i18n::Locale,
        key: &str,
        arg1: impl std::fmt::Display,
        arg2: impl std::fmt::Display,
        arg3: impl std::fmt::Display,
    ) -> String {
        tg(locale, key)
            .replacen("{}", &arg1.to_string(), 1)
            .replacen("{}", &arg2.to_string(), 1)
            .replacen("{}", &arg3.to_string(), 1)
    }
}
mod select {
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
}
mod text;

pub(super) use format::{tg_fmt, tg_fmt2, tg_fmt3};
pub(super) use select::{locale_prefers_chinese, telegram_locale};
pub(super) use text::tg;
