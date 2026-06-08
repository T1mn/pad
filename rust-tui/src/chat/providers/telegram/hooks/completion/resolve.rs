use super::model::ResolvedPendingResult;
use crate::chat::providers::telegram::PendingRequest;
use crate::hook::HookEvent;
use crate::log_debug;
use std::time::Duration;
use tokio::time::sleep;

pub(super) async fn await_pending_result_text(
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

pub(in crate::chat::providers::telegram::hooks) fn resolve_pending_result_text(
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
