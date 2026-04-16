use super::*;

pub(crate) fn pending_sent_ms(pending: &PendingRequest) -> i64 {
    if pending.sent_at_ms > 0 {
        pending.sent_at_ms
    } else {
        pending.sent_at.saturating_mul(1000)
    }
}

pub(crate) fn pending_accepted_ms(pending: &PendingRequest) -> i64 {
    pending.accepted_at_ms.unwrap_or_else(|| {
        pending
            .accepted_at
            .unwrap_or(pending.sent_at)
            .saturating_mul(1000)
    })
}
