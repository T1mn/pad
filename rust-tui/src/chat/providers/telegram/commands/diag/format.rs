use super::super::*;

pub(super) fn format_session_diag_message(
    locale: crate::i18n::Locale,
    context: &SessionDiagContext,
) -> String {
    let mut lines = vec![
        tg(locale, "diag.title").to_string(),
        context.target_label.clone(),
    ];
    if let Some(request_id) = context
        .request_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.request"), request_id));
    }
    if let Some(pane_id) = context.pane_id.as_deref().filter(|value| !value.is_empty()) {
        lines.push(format!("{}: {}", tg(locale, "meta.pane"), pane_id));
    }
    if let Some(session_id) = context
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "meta.session"), session_id));
    }

    if let Some(snapshot) = context.continuity.as_ref() {
        lines.extend(super::super::pending::continuity_detail_lines(
            locale, snapshot,
        ));
    } else {
        lines.push(tg(locale, "diag.empty").to_string());
        if let Some(path) = context
            .transcript_path
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("{}: {}", tg(locale, "diag.transcript"), path));
        }
    }

    lines.join("\n")
}
