use super::diagnostics::{
    append_cache_write_diagnostic, append_hook_event_diagnostic,
    append_preview_assessment_diagnostic,
};
use super::health::{clear_bootstrap_if_resolved, recompute_record_health};
use super::model::{
    ContinuityAttemptClassification, ContinuityWriteSource, PreviewFallbackDecision,
};
use super::storage::mutate_record;
use super::utils::{clean_text, now_ts, observe_transcript};
use crate::hook::HookEvent;
use crate::model::AgentType;
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

pub fn record_cache_write(
    agent_type: &AgentType,
    session_id: &str,
    transcript_path: Option<&Path>,
    source: ContinuityWriteSource,
    turn_count: usize,
) {
    let now = now_ts();
    let record = mutate_record(session_id, now, |record| {
        record.agent_type = Some(agent_type.to_string());
        record.updated_at = now;
        match source {
            ContinuityWriteSource::Hook => {
                record.last_hook_cache_persist_at = Some(now);
            }
            ContinuityWriteSource::Resolver => {
                record.last_resolver_sync_at = Some(now);
            }
        }
        observe_transcript(record, transcript_path, now);
        clear_bootstrap_if_resolved(record);
        recompute_record_health(record);
    });

    if let Some(record) = record {
        append_cache_write_diagnostic(now, source, turn_count, &record);
    }
}

pub fn record_preview_assessment(
    agent_type: &AgentType,
    session_id: Option<&str>,
    transcript_path: Option<&Path>,
    thread_updated_at: Option<i64>,
    cached_turn_count: usize,
    transcript_turn_count: usize,
    decision: &PreviewFallbackDecision,
) {
    let Some(session_id) = clean_text(session_id) else {
        return;
    };
    let now = now_ts();
    let record = mutate_record(session_id, now, |record| {
        record.agent_type = Some(agent_type.to_string());
        record.updated_at = now;
        if let Some(thread_updated_at) = thread_updated_at {
            record.last_thread_updated_at = Some(match record.last_thread_updated_at {
                Some(existing) => existing.max(thread_updated_at),
                None => thread_updated_at,
            });
        }
        observe_transcript(record, transcript_path, now);
        recompute_record_health(record);
    });

    if let Some(record) = record {
        append_preview_assessment_diagnostic(
            now,
            cached_turn_count,
            transcript_turn_count,
            decision,
            &record,
        );
    }
}
