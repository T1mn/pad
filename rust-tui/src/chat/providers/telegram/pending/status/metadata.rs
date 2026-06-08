use super::super::*;

pub(super) fn pending_metadata_lines(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    include_turn: bool,
) -> Vec<String> {
    let mut lines = vec![
        format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
        format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
    ];
    if let Some(session_id) = pending
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
    }
    if include_turn {
        if let Some(turn_id) = pending.turn_id.as_deref().filter(|value| !value.is_empty()) {
            lines.push(format!("{}: {}", tg(locale, "meta.turn"), turn_id));
        }
    }
    if !pending.working_dir.trim().is_empty() {
        lines.push(format!(
            "{}: {}",
            tg(locale, "meta.dir"),
            pending.working_dir
        ));
    }
    lines
}
