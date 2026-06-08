use super::super::*;

pub(crate) fn build_pad_status_body(
    locale: crate::i18n::Locale,
    pad_status: &str,
    state: &TelegramState,
) -> String {
    let target = state
        .selected_target
        .as_ref()
        .map(|target| target.label.clone())
        .unwrap_or_else(|| tg(locale, "status.none").to_string());
    let pending = if state.pending_requests.is_empty() {
        tg(locale, "status.pending_none").to_string()
    } else {
        state
            .pending_requests
            .iter()
            .map(|pending| pending_status_summary_line(locale, pending))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "{}: {}\n{}: {}\n{}:\n{}",
        tg(locale, "status.pad"),
        pad_status,
        tg(locale, "status.target"),
        target,
        tg(locale, "status.pending"),
        pending
    )
}
