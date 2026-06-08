use super::super::*;

pub(super) fn format_session_diag_message(
    locale: crate::i18n::Locale,
    context: &SessionDiagContext,
) -> String {
    let mut body = String::new();
    push_diag_line(&mut body, tg(locale, "diag.title"));
    push_diag_line(&mut body, &context.target_label);
    if let Some(request_id) = context
        .request_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        push_diag_line(
            &mut body,
            &format!("{}: {}", tg(locale, "meta.request"), request_id),
        );
    }
    if let Some(pane_id) = context.pane_id.as_deref().filter(|value| !value.is_empty()) {
        push_diag_line(
            &mut body,
            &format!("{}: {}", tg(locale, "meta.pane"), pane_id),
        );
    }
    if let Some(session_id) = context
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        push_diag_line(
            &mut body,
            &format!("{}: {}", tg(locale, "meta.session"), session_id),
        );
    }

    if let Some(snapshot) = context.continuity.as_ref() {
        for line in super::super::pending::continuity_detail_lines(locale, snapshot) {
            push_diag_line(&mut body, &line);
        }
    } else {
        push_diag_line(&mut body, tg(locale, "diag.empty"));
        if let Some(path) = context
            .transcript_path
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            push_diag_line(
                &mut body,
                &format!("{}: {}", tg(locale, "diag.transcript"), path),
            );
        }
    }

    body
}

fn push_diag_line(out: &mut String, line: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(line);
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
