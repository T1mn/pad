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
mod model;
mod resolve;

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
