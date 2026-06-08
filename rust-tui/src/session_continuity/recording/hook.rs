use crate::hook::HookEvent;
use crate::model::AgentType;
use crate::session_continuity::diagnostics::append_hook_event_diagnostic;
use crate::session_continuity::health::{clear_bootstrap_if_resolved, recompute_record_health};
use crate::session_continuity::model::ContinuityAttemptClassification;
use crate::session_continuity::storage::mutate_record;
use crate::session_continuity::utils::{clean_text, now_ts, observe_transcript};
use std::path::Path;

pub fn record_hook_event(
    agent_type: Option<&AgentType>,
    event: &HookEvent,
    fallback_session_id: Option<&str>,
    fallback_transcript_path: Option<&str>,
) {
    let Some(session_id) = clean_text(event.session_id.as_deref().or(fallback_session_id)) else {
        return;
    };
    let transcript_path = clean_text(
        event
            .transcript_path
            .as_deref()
            .or(fallback_transcript_path),
    )
    .map(str::to_owned);
    let now = now_ts();

    let record = mutate_record(session_id, now, |record| {
        if let Some(agent_type) = agent_type {
            record.agent_type = Some(agent_type.to_string());
        }
        record.last_hook_event = Some(event.event.clone());
        record.last_hook_event_at = Some(now);
        record.last_turn_id = clean_text(event.turn_id.as_deref()).map(str::to_owned);
        record.updated_at = now;

        match event.event.as_str() {
            "session_start" if transcript_path.is_none() => {
                record.attempt_classification =
                    ContinuityAttemptClassification::TransientResumeBootstrap;
                record.bootstrap_event_count = record.bootstrap_event_count.saturating_add(1);
            }
            "user_prompt_submit" => {
                record.last_prompt_submit_at = Some(now);
            }
            "stop" => {
                record.last_stop_at = Some(now);
                if clean_text(event.last_assistant_message.as_deref()).is_some() {
                    record.last_assistant_message_at = Some(now);
                }
            }
            _ => {}
        }

        observe_transcript(record, transcript_path.as_deref().map(Path::new), now);
        clear_bootstrap_if_resolved(record);
        recompute_record_health(record);
    });

    if let Some(record) = record {
        append_hook_event_diagnostic(now, event, &record);
    }
}
