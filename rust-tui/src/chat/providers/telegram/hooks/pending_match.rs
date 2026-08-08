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
            .map(|prompt| format!("{:x}", md5::compute(prompt.as_bytes())) == pending.prompt_hash)
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
