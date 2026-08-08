mod ids {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::LazyLock;

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
        crate::time::unix_now_ts()
    }

    pub(in crate::chat::providers::telegram) fn now_ms_i64() -> i64 {
        crate::time::unix_now_millis() as i64
    }
}
mod model;
mod pending;
mod storage {
    use super::model::TelegramState;
    use std::fs;
    use std::io;

    pub(in crate::chat::providers::telegram) fn load_state() -> io::Result<TelegramState> {
        let path = crate::paths::telegram_state_path();
        match fs::read_to_string(path) {
            Ok(body) => serde_json::from_str(&body)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(TelegramState::default()),
            Err(err) => Err(err),
        }
    }

    pub(in crate::chat::providers::telegram) fn save_state(
        state: &TelegramState,
    ) -> io::Result<()> {
        let body = serde_json::to_string_pretty(state)?;
        crate::atomic_file::write_private(&crate::paths::telegram_state_path(), body)
    }

    pub(in crate::chat::providers::telegram) fn journal_len() -> u64 {
        fs::metadata(crate::paths::hook_events_path())
            .map(|meta| meta.len())
            .unwrap_or(0)
    }
}

pub(super) use ids::{next_draft_id, next_request_id, now_ms_i64, now_ts};
pub(super) use model::{AgentSnapshotEntry, PendingRequest, SelectedTarget, TelegramState};
pub(super) use pending::{
    mark_update_processed, pending_request_index_by_id, pending_request_index_by_pane,
    remove_pending_request, remove_selected_target_pending_request,
};
pub(super) use storage::{journal_len, load_state, save_state};
