use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_REQUEST_ID: LazyLock<AtomicU64> =
    LazyLock::new(|| AtomicU64::new((now_ms_i64().max(1) as u64).saturating_mul(1000)));
static NEXT_DRAFT_ID: LazyLock<AtomicU64> =
    LazyLock::new(|| AtomicU64::new((now_ms_i64().max(1) as u64).saturating_mul(1000)));

pub(in crate::chat::providers::telegram) fn next_request_id() -> String {
    format!("tg-{}", NEXT_REQUEST_ID.fetch_add(1, Ordering::SeqCst))
}

pub(in crate::chat::providers::telegram) fn next_draft_id() -> i64 {
    NEXT_DRAFT_ID.fetch_add(1, Ordering::SeqCst) as i64
}

pub(in crate::chat::providers::telegram) fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub(in crate::chat::providers::telegram) fn now_ms_i64() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
