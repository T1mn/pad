use super::*;

mod continuity;
mod metadata;

pub(crate) use continuity::continuity_detail_lines;
pub(super) use continuity::continuity_status_line;
use metadata::pending_metadata_lines;

pub(super) fn phase_label(locale: crate::i18n::Locale, phase: &str) -> String {
    match phase {
        "awaiting_submit" => tg(locale, "phase.awaiting_submit").to_string(),
        "awaiting_confirm" => tg(locale, "phase.awaiting_confirm").to_string(),
        "awaiting_stop" => tg(locale, "phase.accepted").to_string(),
        "delivering_result" => tg(locale, "phase.delivering").to_string(),
        _ => phase.to_string(),
    }
}

pub(crate) fn pending_status_summary_line(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
) -> String {
    format!(
        "{} • {} • {} • {}",
        pending.request_id,
        pending.pane_id,
        pending.target_label,
        phase_label(locale, &pending.phase)
    )
}

pub(crate) fn pending_status_text(
    locale: crate::i18n::Locale,
    pending: &PendingRequest,
    now: i64,
) -> String {
    if pending.approval_call_id.is_some() {
        let mut text = String::new();
        push_status_line(&mut text, tg(locale, "phase.awaiting_confirm"));
        push_status_line(&mut text, &pending.target_label);
        for line in pending_metadata_lines(locale, pending, false) {
            push_status_line(&mut text, &line);
        }
        if let Some(justification) = pending.approval_justification.as_deref() {
            push_status_line(&mut text, &truncate_chars(justification, 220));
        }
        return text;
    }

    let headline = match pending.phase.as_str() {
        "awaiting_submit" => tg(locale, "phase.awaiting_submit").to_string(),
        "awaiting_stop" => match pending.accepted_at {
            Some(accepted_at) if now.saturating_sub(accepted_at) >= 4 => {
                tg_fmt(locale, "phase.working", now.saturating_sub(accepted_at))
            }
            _ => tg(locale, "phase.accepted").to_string(),
        },
        "delivering_result" => tg(locale, "phase.delivering").to_string(),
        _ => tg(locale, "phase.completed").to_string(),
    };

    let mut text = String::new();
    push_status_line(&mut text, &headline);
    push_status_line(&mut text, &pending.target_label);
    for line in pending_metadata_lines(locale, pending, false) {
        push_status_line(&mut text, &line);
    }
    if let Some(snapshot) = pending_continuity_snapshot(pending) {
        if snapshot.health != crate::session_continuity::ContinuityHealth::Healthy
            || snapshot.attempt_classification
                != crate::session_continuity::ContinuityAttemptClassification::Normal
        {
            push_status_line(&mut text, &continuity_status_line(locale, &snapshot));
        }
    }
    text
}

fn push_status_line(out: &mut String, line: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(line);
}

fn pending_continuity_snapshot(
    pending: &PendingRequest,
) -> Option<crate::session_continuity::ContinuitySnapshot> {
    crate::session_continuity::load_snapshot_for(
        pending.session_id.as_deref(),
        pending.transcript_path.as_deref(),
    )
}
