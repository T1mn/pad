use super::super::*;

pub(in crate::chat::providers::telegram::pending) fn continuity_status_line(
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
