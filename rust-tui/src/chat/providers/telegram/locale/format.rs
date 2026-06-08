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
