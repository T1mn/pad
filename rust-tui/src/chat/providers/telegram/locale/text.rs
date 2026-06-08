use super::select::locale_prefers_chinese;

mod approval;
mod command;
mod core;
mod status;

pub(in crate::chat::providers::telegram) fn tg(locale: crate::i18n::Locale, key: &str) -> &str {
    let zh = locale_prefers_chinese(locale);
    core::text(key, zh)
        .or_else(|| approval::text(key, zh))
        .or_else(|| status::text(key, zh))
        .or_else(|| command::text(key, zh))
        .unwrap_or(key)
}
