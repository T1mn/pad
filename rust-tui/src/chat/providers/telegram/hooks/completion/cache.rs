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
