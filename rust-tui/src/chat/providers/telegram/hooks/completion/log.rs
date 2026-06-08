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
