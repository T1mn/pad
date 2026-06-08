use crate::model::AgentType;
use crate::session_continuity::diagnostics::append_cache_write_diagnostic;
use crate::session_continuity::health::{clear_bootstrap_if_resolved, recompute_record_health};
use crate::session_continuity::model::ContinuityWriteSource;
use crate::session_continuity::storage::mutate_record;
use crate::session_continuity::utils::{now_ts, observe_transcript};
use std::path::Path;

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
