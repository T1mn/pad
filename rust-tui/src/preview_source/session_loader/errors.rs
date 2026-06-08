use crate::i18n::{self, Locale};

pub(super) fn session_unavailable_message(locale: Locale, detail: &str) -> String {
    format!(
        "{}\n\n{}",
        i18n::t(locale, "preview.session_unavailable"),
        detail
    )
}
