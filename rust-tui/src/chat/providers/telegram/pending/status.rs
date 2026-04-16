use super::*;

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

pub(super) fn continuity_status_line(
    locale: crate::i18n::Locale,
    snapshot: &crate::session_continuity::ContinuitySnapshot,
) -> String {
    let mut parts = vec![continuity_health_text(locale, snapshot.health).to_string()];
    if let Some(lag_seconds) = snapshot.lag_seconds.filter(|lag| *lag > 0) {
        parts.push(tg_fmt(locale, "diag.lag_short", lag_seconds));
    }
    if snapshot.attempt_classification
        != crate::session_continuity::ContinuityAttemptClassification::Normal
    {
        parts.push(continuity_attempt_text(locale, snapshot.attempt_classification).to_string());
    }
    format!("{}: {}", tg(locale, "diag.summary"), parts.join(" · "))
}

pub(crate) fn continuity_detail_lines(
    locale: crate::i18n::Locale,
    snapshot: &crate::session_continuity::ContinuitySnapshot,
) -> Vec<String> {
    let mut lines = vec![
        format!(
            "{}: {}",
            tg(locale, "diag.health"),
            continuity_health_text(locale, snapshot.health)
        ),
        format!(
            "{}: {}",
            tg(locale, "diag.classification"),
            continuity_attempt_text(locale, snapshot.attempt_classification)
        ),
    ];
    if let Some(lag_seconds) = snapshot.lag_seconds {
        lines.push(format!(
            "{}: {}",
            tg(locale, "diag.lag"),
            tg_fmt(locale, "diag.lag_short", lag_seconds)
        ));
    }
    if let Some(event) = snapshot
        .last_hook_event
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "diag.last_hook"), event));
    }
    if snapshot.stale_event_count > 0 {
        lines.push(format!(
            "{}: {}",
            tg(locale, "diag.stale_events"),
            snapshot.stale_event_count
        ));
    }
    if let Some(path) = snapshot
        .transcript_path
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("{}: {}", tg(locale, "diag.transcript"), path));
    }
    lines
}

fn pending_metadata_lines(
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

fn pending_continuity_snapshot(
    pending: &PendingRequest,
) -> Option<crate::session_continuity::ContinuitySnapshot> {
    crate::session_continuity::load_snapshot_for(
        pending.session_id.as_deref(),
        pending.transcript_path.as_deref(),
    )
}

fn continuity_health_text(
    locale: crate::i18n::Locale,
    health: crate::session_continuity::ContinuityHealth,
) -> &'static str {
    match (locale_prefers_chinese(locale), health) {
        (true, crate::session_continuity::ContinuityHealth::Healthy) => "健康",
        (true, crate::session_continuity::ContinuityHealth::Lagging) => "滞后",
        (true, crate::session_continuity::ContinuityHealth::Frozen) => "冻结",
        (false, crate::session_continuity::ContinuityHealth::Healthy) => "healthy",
        (false, crate::session_continuity::ContinuityHealth::Lagging) => "lagging",
        (false, crate::session_continuity::ContinuityHealth::Frozen) => "frozen",
    }
}

fn continuity_attempt_text(
    locale: crate::i18n::Locale,
    attempt: crate::session_continuity::ContinuityAttemptClassification,
) -> &'static str {
    match (locale_prefers_chinese(locale), attempt) {
        (true, crate::session_continuity::ContinuityAttemptClassification::Normal) => "正常",
        (
            true,
            crate::session_continuity::ContinuityAttemptClassification::TransientResumeBootstrap,
        ) => "短暂 resume 引导",
        (false, crate::session_continuity::ContinuityAttemptClassification::Normal) => "normal",
        (
            false,
            crate::session_continuity::ContinuityAttemptClassification::TransientResumeBootstrap,
        ) => "transient_resume_bootstrap",
    }
}
