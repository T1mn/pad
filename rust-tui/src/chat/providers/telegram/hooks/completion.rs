use super::*;

pub(super) async fn complete_pending_request(
    config: &Config,
    state: &mut TelegramState,
    request_id: &str,
    pending_snapshot: &PendingRequest,
    event: &HookEvent,
    locale: crate::i18n::Locale,
) -> PendingCompletionOutcome {
    let resolved = await_pending_result_text(pending_snapshot, event).await;
    cache_pending_completion(
        pending_request_index_by_id(state, request_id)
            .and_then(|index| state.pending_requests.get_mut(index)),
        locale,
        &resolved,
    );
    match deliver_pending_result(config, state, locale, request_id).await {
        Ok(()) => PendingCompletionOutcome::delivered(&resolved),
        Err(err) => PendingCompletionOutcome::deferred(&resolved, err.to_string()),
    }
}

fn cache_pending_completion(
    pending: Option<&mut PendingRequest>,
    locale: crate::i18n::Locale,
    resolved: &ResolvedPendingResult,
) {
    let Some(pending) = pending else {
        return;
    };
    pending.phase = "delivering_result".to_string();
    pending.completed_text = Some(
        resolved
            .text
            .clone()
            .unwrap_or_else(|| tg(locale, "result.missing").to_string()),
    );
    pending.completed_source = Some(resolved.source.to_string());
    pending.delivery_retry_at = 0;
    pending.last_status_at = None;
}

pub(super) fn log_pending_completion(
    channel: &str,
    pending_snapshot: &PendingRequest,
    completion: &PendingCompletionOutcome,
) {
    if let Some(err) = completion.error.as_deref() {
        log_debug!(
            "telegram: {} deferred result delivery request {} total_ms={} run_ms={} result_source={} result_chars={} err={}",
            channel,
            pending_snapshot.request_id,
            now_ms_i64().saturating_sub(pending_sent_ms(pending_snapshot)),
            now_ms_i64().saturating_sub(pending_accepted_ms(pending_snapshot)),
            completion.source,
            completion.char_count,
            err
        );
    } else {
        log_debug!(
            "telegram: {} completed request {} total_ms={} run_ms={} result_source={} result_chars={}",
            channel,
            pending_snapshot.request_id,
            now_ms_i64().saturating_sub(pending_sent_ms(pending_snapshot)),
            now_ms_i64().saturating_sub(pending_accepted_ms(pending_snapshot)),
            completion.source,
            completion.char_count
        );
    }
}

async fn await_pending_result_text(
    pending: &PendingRequest,
    event: &HookEvent,
) -> ResolvedPendingResult {
    let initial = resolve_pending_result_text(pending, event);
    if pending.agent_kind != "codex" || initial.source == "transcript_completion" {
        return initial;
    }

    // Codex can emit Stop before the final assistant message is appended to the
    // transcript. Give the transcript a short window to catch up before falling
    // back to the hook payload.
    const RETRIES: usize = 24;
    const SLEEP_MS: u64 = 250;
    for attempt in 1..=RETRIES {
        sleep(Duration::from_millis(SLEEP_MS)).await;
        let retried = resolve_pending_result_text(pending, event);
        if retried.source == "transcript_completion" {
            log_debug!(
                "telegram: codex transcript caught up pane={} after_retry={} wait_ms={}",
                pending.pane_id,
                attempt,
                attempt as u64 * SLEEP_MS
            );
            return retried;
        }
    }

    log_debug!(
        "telegram: codex transcript still missing pane={} after_wait_ms={} fallback_source={}",
        pending.pane_id,
        RETRIES as u64 * SLEEP_MS,
        initial.source
    );
    initial
}

pub(super) fn resolve_pending_result_text(
    pending: &PendingRequest,
    event: &HookEvent,
) -> ResolvedPendingResult {
    let hook_text = event
        .last_assistant_message
        .clone()
        .filter(|text| !text.trim().is_empty());
    let transcript_text = pending.transcript_path.as_deref().and_then(|path| {
        crate::chat::approval::scan_codex_answer_updates(
            std::path::Path::new(path),
            pending.result_scan_offset,
            pending.turn_id.as_deref().or(event.turn_id.as_deref()),
        )
        .ok()
        .flatten()
    });

    if pending.agent_kind == "codex" {
        if let (Some(hook), Some(transcript)) = (hook_text.as_deref(), transcript_text.as_deref()) {
            if hook.trim() != transcript.trim() {
                log_debug!(
                    "telegram: codex stop payload mismatch pane={} hook_chars={} transcript_chars={} preferring=transcript_completion",
                    pending.pane_id,
                    hook.chars().count(),
                    transcript.chars().count()
                );
            }
        }
        if transcript_text.is_some() {
            return ResolvedPendingResult::new(transcript_text, "transcript_completion");
        }
        if hook_text.is_some() {
            return ResolvedPendingResult::new(hook_text, "hook_payload");
        }
        return ResolvedPendingResult::new(None, "missing");
    }

    if hook_text.is_some() {
        ResolvedPendingResult::new(hook_text, "hook_payload")
    } else if transcript_text.is_some() {
        ResolvedPendingResult::new(transcript_text, "transcript_delta")
    } else {
        ResolvedPendingResult::new(None, "missing")
    }
}

pub(super) struct ResolvedPendingResult {
    pub(super) text: Option<String>,
    pub(super) source: &'static str,
    char_count: usize,
}

impl ResolvedPendingResult {
    fn new(text: Option<String>, source: &'static str) -> Self {
        let char_count = text
            .as_ref()
            .map(|value| value.chars().count())
            .unwrap_or(0);
        Self {
            text,
            source,
            char_count,
        }
    }
}

pub(super) struct PendingCompletionOutcome {
    source: &'static str,
    char_count: usize,
    error: Option<String>,
}

impl PendingCompletionOutcome {
    fn delivered(resolved: &ResolvedPendingResult) -> Self {
        Self {
            source: resolved.source,
            char_count: resolved.char_count,
            error: None,
        }
    }

    fn deferred(resolved: &ResolvedPendingResult, error: String) -> Self {
        Self {
            source: resolved.source,
            char_count: resolved.char_count,
            error: Some(error),
        }
    }
}
