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
        let mut lines = vec![
            tg(locale, "phase.awaiting_confirm").to_string(),
            pending.target_label.clone(),
        ];
        lines.extend(pending_metadata_lines(locale, pending, false));
        if let Some(justification) = pending.approval_justification.as_deref() {
            lines.push(truncate_chars(justification, 220));
        }
        return lines.join("\n");
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

    let mut lines = vec![headline, pending.target_label.clone()];
    lines.extend(pending_metadata_lines(locale, pending, false));
    if let Some(snapshot) = pending_continuity_snapshot(pending) {
        if snapshot.health != crate::session_continuity::ContinuityHealth::Healthy
            || snapshot.attempt_classification
                != crate::session_continuity::ContinuityAttemptClassification::Normal
        {
            lines.push(continuity_status_line(locale, &snapshot));
        }
    }
    lines.join("\n")
}

fn pending_continuity_snapshot(
    pending: &PendingRequest,
) -> Option<crate::session_continuity::ContinuitySnapshot> {
    crate::session_continuity::load_snapshot_for(
        pending.session_id.as_deref(),
        pending.transcript_path.as_deref(),
    )
}
