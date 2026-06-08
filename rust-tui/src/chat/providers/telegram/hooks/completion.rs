mod cache;
mod log;
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
