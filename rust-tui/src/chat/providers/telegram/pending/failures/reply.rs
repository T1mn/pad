use super::super::*;

pub(in crate::chat::providers::telegram::pending) fn pending_failure_reply_text(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    failure: &crate::chat::approval::CodexFailureEvent,
    continuity: Option<&crate::session_continuity::ContinuitySnapshot>,
) -> String {
    let mut lines = vec![
        tg(locale, "failure.title").to_string(),
        format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
        format!("{}: {}", tg(locale, "meta.target"), pending.target_label),
        format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
    ];
    if let Some(session_id) = pending
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
    }
    if let Some(turn_id) = pending.turn_id.as_deref().filter(|value| !value.is_empty()) {
        lines.push(format!("{}: {}", tg(locale, "meta.turn"), turn_id));
    }
    if !pending.working_dir.trim().is_empty() {
        lines.push(format!(
            "{}: {}",
            tg(locale, "meta.dir"),
            pending.working_dir
        ));
    }
    if let Some(error_info) = failure
        .error_info
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "failure.kind"), error_info));
    }
    if let Some(snapshot) = continuity {
        lines.extend(continuity_detail_lines(locale, snapshot));
    }

    format!(
        "{}\n\n{}:\n{}",
        lines.join("\n"),
        tg(locale, "failure.detail"),
        failure.message
    )
}
