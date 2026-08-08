use super::*;

mod completion {
    mod cache {
        use super::model::ResolvedPendingResult;
        use crate::chat::providers::telegram::{locale::tg, PendingRequest};

        pub(super) fn cache_pending_completion(
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
    }
    mod log {
        use super::model::PendingCompletionOutcome;
        use crate::chat::providers::telegram::{
            now_ms_i64, pending_accepted_ms, pending_sent_ms, PendingRequest,
        };
        use crate::log_debug;

        pub(in crate::chat::providers::telegram::hooks) fn log_pending_completion(
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
    }
    mod model {
        pub(in crate::chat::providers::telegram::hooks) struct ResolvedPendingResult {
            pub(in crate::chat::providers::telegram::hooks) text: Option<String>,
            pub(in crate::chat::providers::telegram::hooks) source: &'static str,
            pub(in crate::chat::providers::telegram::hooks) char_count: usize,
        }

        impl ResolvedPendingResult {
            pub(super) fn new(text: Option<String>, source: &'static str) -> Self {
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

        pub(in crate::chat::providers::telegram::hooks) struct PendingCompletionOutcome {
            pub(in crate::chat::providers::telegram::hooks) source: &'static str,
            pub(in crate::chat::providers::telegram::hooks) char_count: usize,
            pub(in crate::chat::providers::telegram::hooks) error: Option<String>,
        }

        impl PendingCompletionOutcome {
            pub(super) fn delivered(resolved: &ResolvedPendingResult) -> Self {
                Self {
                    source: resolved.source,
                    char_count: resolved.char_count,
                    error: None,
                }
            }

            pub(super) fn deferred(resolved: &ResolvedPendingResult, error: String) -> Self {
                Self {
                    source: resolved.source,
                    char_count: resolved.char_count,
                    error: Some(error),
                }
            }
        }
    }
    mod resolve {
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
                if let (Some(hook), Some(transcript)) =
                    (hook_text.as_deref(), transcript_text.as_deref())
                {
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
    }

    use super::*;
    use cache::cache_pending_completion;
    pub(super) use log::log_pending_completion;
    pub(super) use model::PendingCompletionOutcome;
    use resolve::await_pending_result_text;
    #[cfg(test)]
    pub(super) use resolve::resolve_pending_result_text;

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
}
mod direct;
mod journal {
    use super::direct::daemon_socket_is_active;
    use super::*;

    fn sync_state_from_disk(state: &mut TelegramState) {
        if let Ok(mut latest) = load_state() {
            latest.journal_position = latest.journal_position.max(state.journal_position);
            latest.last_journal_recovery_at = latest
                .last_journal_recovery_at
                .max(state.last_journal_recovery_at);
            *state = latest;
        }
    }

    pub(in crate::chat::providers::telegram) fn sync_state_from_disk_public(
        state: &mut TelegramState,
    ) {
        sync_state_from_disk(state);
    }

    pub(in crate::chat::providers::telegram) fn should_probe_hook_journal(
        state: &TelegramState,
    ) -> bool {
        should_probe_hook_journal_inner(state, daemon_socket_is_active(), now_ts())
    }

    pub(in crate::chat::providers::telegram) fn should_probe_hook_journal_inner(
        state: &TelegramState,
        direct_hook_active: bool,
        now: i64,
    ) -> bool {
        if state.pending_requests.is_empty() {
            return false;
        }
        if state.last_journal_recovery_at == 0 {
            return true;
        }
        if !direct_hook_active {
            return now.saturating_sub(state.last_journal_recovery_at) >= 1;
        }
        if now.saturating_sub(state.last_journal_recovery_at) < JOURNAL_RECOVERY_RETRY_SECS {
            return false;
        }
        state
            .pending_requests
            .iter()
            .any(|pending| match pending.phase.as_str() {
                "awaiting_submit" => {
                    now.saturating_sub(pending.sent_at) >= JOURNAL_RECOVERY_STALL_SECS
                }
                "awaiting_stop" | "awaiting_confirm" => {
                    now.saturating_sub(pending.accepted_at.unwrap_or(pending.sent_at))
                        >= JOURNAL_RECOVERY_STALL_SECS
                }
                _ => false,
            })
    }

    pub(in crate::chat::providers::telegram) fn remember_processed_hook_event(
        state: &mut TelegramState,
        event: &HookEvent,
    ) -> bool {
        let signature = hook_event_signature(event);
        if recent_hook_signature_exists(&signature) {
            return false;
        }
        if state
            .processed_hook_signatures
            .iter()
            .any(|existing| existing == &signature)
        {
            return false;
        }
        state.processed_hook_signatures.push(signature);
        const MAX_PROCESSED_HOOKS: usize = 64;
        if state.processed_hook_signatures.len() > MAX_PROCESSED_HOOKS {
            let drop_count = state.processed_hook_signatures.len() - MAX_PROCESSED_HOOKS;
            state.processed_hook_signatures.drain(0..drop_count);
        }
        remember_recent_hook_signature(
            state
                .processed_hook_signatures
                .last()
                .expect("processed hook signature must exist"),
        );
        true
    }

    fn hook_event_signature(event: &HookEvent) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            event.event,
            event.terminal.pane_id.as_deref().unwrap_or(""),
            event.turn_id.as_deref().unwrap_or(""),
            event.timestamp.as_deref().unwrap_or(""),
            event.session_id.as_deref().unwrap_or(""),
            event
                .prompt
                .as_deref()
                .map(|prompt| format!("{:x}", md5::compute(prompt.as_bytes())))
                .unwrap_or_default()
        )
    }

    fn recent_hook_signature_exists(signature: &str) -> bool {
        RECENT_HOOK_SIGNATURES
            .lock()
            .map(|signatures| signatures.iter().any(|existing| existing == signature))
            .unwrap_or(false)
    }

    fn remember_recent_hook_signature(signature: &str) {
        if let Ok(mut signatures) = RECENT_HOOK_SIGNATURES.lock() {
            signatures.push(signature.to_string());
            const MAX_RECENT_HOOK_SIGNATURES: usize = 128;
            if signatures.len() > MAX_RECENT_HOOK_SIGNATURES {
                let drop_count = signatures.len() - MAX_RECENT_HOOK_SIGNATURES;
                signatures.drain(0..drop_count);
            }
        }
    }
}
mod pending_match {
    mod advance {
        use super::super::*;

        pub(in crate::chat::providers::telegram::hooks) fn advance_pending_to_awaiting_stop(
            pending: Option<&mut PendingRequest>,
            event: &HookEvent,
            record_accepted_at_ms: bool,
        ) {
            let Some(pending) = pending else {
                return;
            };
            pending.phase = "awaiting_stop".to_string();
            pending.accepted_at = Some(now_ts());
            if record_accepted_at_ms {
                pending.accepted_at_ms = Some(now_ms_i64());
            }
            if event.turn_id.is_some() {
                pending.turn_id = event.turn_id.clone();
            }
            if event.session_id.is_some() {
                pending.session_id = event.session_id.clone();
            }
            if event.cwd.is_some() {
                pending.working_dir = event.cwd.clone().unwrap_or_default();
            }
            if event.transcript_path.is_some() {
                pending.transcript_path = event.transcript_path.clone();
            }
            if pending.result_scan_offset == 0 {
                if let Some(path) = pending.transcript_path.as_deref() {
                    pending.result_scan_offset = transcript_len(path);
                }
            }
            if pending.failure_scan_offset == 0 {
                if let Some(path) = pending.transcript_path.as_deref() {
                    pending.failure_scan_offset = transcript_len(path);
                }
            }
        }
    }
    mod apply {
        use super::super::completion::{complete_pending_request, log_pending_completion};
        use super::super::*;
        use super::advance::advance_pending_to_awaiting_stop;
        use super::matching::matching_pending_request_index;

        pub(in crate::chat::providers::telegram) async fn apply_hook_event_to_pending(
            config: &Config,
            state: &mut TelegramState,
            event: &HookEvent,
        ) -> TelegramResult<bool> {
            let locale = telegram_locale(config);
            let Some(pending_index) = matching_pending_request_index(state, event) else {
                return Ok(false);
            };
            let pending_snapshot = state.pending_requests[pending_index].clone();
            match event.event.as_str() {
                "user_prompt_submit" => {
                    advance_pending_to_awaiting_stop(
                        state.pending_requests.get_mut(pending_index),
                        event,
                        true,
                    );
                    refresh_pending_feedback(config, state, true);
                    log_debug!(
                        "telegram: pending request {} reached awaiting_stop dispatch_to_submit_ms={}",
                        pending_snapshot.request_id,
                        now_ms_i64().saturating_sub(pending_sent_ms(&pending_snapshot))
                    );
                    Ok(false)
                }
                "stop" => {
                    let completion = complete_pending_request(
                        config,
                        state,
                        &pending_snapshot.request_id,
                        &pending_snapshot,
                        event,
                        locale,
                    )
                    .await;
                    log_pending_completion("journal", &pending_snapshot, &completion);
                    Ok(true)
                }
                _ => Ok(false),
            }
        }
    }
    mod matching {
        use super::super::*;

        pub(in crate::chat::providers::telegram) fn matching_pending_request_index(
            state: &TelegramState,
            event: &HookEvent,
        ) -> Option<usize> {
            let pane_id = event.terminal.pane_id.as_deref()?;
            let pending_index = pending_request_index_by_pane(state, pane_id)?;
            let pending = state.pending_requests.get(pending_index)?;
            match event.event.as_str() {
                "user_prompt_submit" if pending_matches_submit_prompt(pending, event) => {
                    Some(pending_index)
                }
                "stop" if pending_can_complete_from_stop(pending, event) => Some(pending_index),
                _ => None,
            }
        }

        pub(in crate::chat::providers::telegram) fn pending_can_complete_from_stop(
            pending: &PendingRequest,
            event: &HookEvent,
        ) -> bool {
            matches!(pending.phase.as_str(), "awaiting_stop" | "awaiting_confirm")
                && hook_event_matches_pending_turn(pending, event)
        }

        pub(in crate::chat::providers::telegram) fn hook_event_matches_pending_turn(
            pending: &PendingRequest,
            event: &HookEvent,
        ) -> bool {
            match (pending.turn_id.as_deref(), event.turn_id.as_deref()) {
                (Some(_), None) if pending.agent_kind == "codex" => false,
                (Some(expected), Some(actual)) => expected == actual,
                _ => true,
            }
        }

        pub(in crate::chat::providers::telegram::hooks) fn pending_matches_submit_prompt(
            pending: &PendingRequest,
            event: &HookEvent,
        ) -> bool {
            event
                .prompt
                .as_deref()
                .map(|prompt| {
                    format!("{:x}", md5::compute(prompt.as_bytes())) == pending.prompt_hash
                })
                .unwrap_or(true)
        }
    }

    pub(super) use advance::advance_pending_to_awaiting_stop;
    pub(in crate::chat::providers::telegram) use apply::apply_hook_event_to_pending;
    pub(in crate::chat::providers::telegram) use matching::pending_can_complete_from_stop;
    pub(super) use matching::pending_matches_submit_prompt;
    #[cfg(test)]
    pub(in crate::chat::providers::telegram) use matching::{
        hook_event_matches_pending_turn, matching_pending_request_index,
    };
}

#[cfg(test)]
use completion::resolve_pending_result_text;
pub(super) use direct::{daemon_socket_is_active, start_direct_hook_listener};
#[cfg(test)]
pub(super) use journal::should_probe_hook_journal_inner;
pub(super) use journal::{
    remember_processed_hook_event, should_probe_hook_journal, sync_state_from_disk_public,
};
pub(super) use pending_match::apply_hook_event_to_pending;
#[cfg(test)]
pub(super) use pending_match::{
    hook_event_matches_pending_turn, matching_pending_request_index, pending_can_complete_from_stop,
};

#[cfg(test)]
mod tests;
