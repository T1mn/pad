use crate::preview_source::PreviewRequest;
use std::time::Duration;

pub(super) fn log_prefer_cache(request: &PreviewRequest, reason: &str, lag_seconds: Option<i64>) {
    log_debug!(
        "preview.session: target={} agent={} continuity={} lag_s={} prefer_cache=1",
        request.target_key,
        request.agent_type,
        reason,
        lag_seconds.unwrap_or_default()
    );
}

pub(super) fn log_parse_success_if_slow(
    request: &PreviewRequest,
    resolve_elapsed: Duration,
    parse_elapsed: Duration,
    total_elapsed: Duration,
    turns_len: usize,
) {
    if total_elapsed.as_millis() >= 40 {
        log_debug!(
            "preview.session: target={} agent={} resolve_ms={} parse_ms={} turns={} total_ms={}",
            request.target_key,
            request.agent_type,
            resolve_elapsed.as_millis(),
            parse_elapsed.as_millis(),
            turns_len,
            total_elapsed.as_millis()
        );
    }
}

pub(super) fn log_empty_parse_if_slow(
    request: &PreviewRequest,
    resolve_elapsed: Duration,
    parse_elapsed: Duration,
    total_elapsed: Duration,
) {
    if total_elapsed.as_millis() >= 40 {
        log_debug!(
            "preview.session: target={} agent={} resolve_ms={} parse_ms={} turns=0 total_ms={}",
            request.target_key,
            request.agent_type,
            resolve_elapsed.as_millis(),
            parse_elapsed.as_millis(),
            total_elapsed.as_millis()
        );
    }
}

pub(super) fn log_parse_error_if_slow(
    request: &PreviewRequest,
    resolve_elapsed: Duration,
    parse_elapsed: Duration,
    total_elapsed: Duration,
) {
    if total_elapsed.as_millis() >= 40 {
        log_debug!(
            "preview.session: target={} agent={} resolve_ms={} parse_ms={} error=1 total_ms={}",
            request.target_key,
            request.agent_type,
            resolve_elapsed.as_millis(),
            parse_elapsed.as_millis(),
            total_elapsed.as_millis()
        );
    }
}

pub(super) fn log_missing_target_if_slow(
    request: &PreviewRequest,
    resolve_elapsed: Duration,
    total_elapsed: Duration,
) {
    if total_elapsed.as_millis() >= 20 {
        log_debug!(
            "preview.session: target={} agent={} missing_target=1 resolve_ms={} total_ms={}",
            request.target_key,
            request.agent_type,
            resolve_elapsed.as_millis(),
            total_elapsed.as_millis()
        );
    }
}
