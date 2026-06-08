use crate::model::AgentType;
use crate::session_continuity::diagnostics::append_preview_assessment_diagnostic;
use crate::session_continuity::health::recompute_record_health;
use crate::session_continuity::model::PreviewFallbackDecision;
use crate::session_continuity::storage::mutate_record;
use crate::session_continuity::utils::{clean_text, now_ts, observe_transcript};
use std::path::Path;

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
