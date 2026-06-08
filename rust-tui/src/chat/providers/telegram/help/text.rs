mod codex;
mod common;
mod overview;
mod workflow;

pub(super) fn help_text(locale: crate::i18n::Locale, key: &str) -> &'static str {
    common::text(locale, key)
        .or_else(|| overview::text(locale, key))
        .or_else(|| codex::text(locale, key))
        .or_else(|| workflow::text(locale, key))
        .unwrap_or("")
}
