use super::super::*;

pub(in crate::chat::providers::telegram::pending) fn pending_failure_reply_text(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    failure: &crate::chat::approval::CodexFailureEvent,
    continuity: Option<&crate::session_continuity::ContinuitySnapshot>,
) -> String {
    let mut reply = String::new();
    push_failure_reply_line(&mut reply, tg(locale, "failure.title"));
    push_failure_reply_line(
        &mut reply,
        &format!("{}: {}", tg(locale, "meta.request"), pending.request_id),
    );
    push_failure_reply_line(
        &mut reply,
        &format!("{}: {}", tg(locale, "meta.target"), pending.target_label),
    );
    push_failure_reply_line(
        &mut reply,
        &format!("{}: {}", tg(locale, "meta.pane"), pending.pane_id),
    );
    if let Some(session_id) = pending
        .session_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        push_failure_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "meta.session"), session_id),
        );
    }
    if let Some(turn_id) = pending.turn_id.as_deref().filter(|value| !value.is_empty()) {
        push_failure_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "meta.turn"), turn_id),
        );
    }
    if !pending.working_dir.trim().is_empty() {
        push_failure_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "meta.dir"), pending.working_dir),
        );
    }
    if let Some(error_info) = failure
        .error_info
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        push_failure_reply_line(
            &mut reply,
            &format!("{}: {}", tg(locale, "failure.kind"), error_info),
        );
    }
    if let Some(snapshot) = continuity {
        for line in continuity_detail_lines(locale, snapshot) {
            push_failure_reply_line(&mut reply, &line);
        }
    }

    reply.push_str("\n\n");
    reply.push_str(tg(locale, "failure.detail"));
    reply.push_str(":\n");
    reply.push_str(&failure.message);
    reply
}

fn push_failure_reply_line(out: &mut String, line: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(line);
}
