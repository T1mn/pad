use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct TelegramState {
    pub(super) update_offset: i64,
    pub(super) last_processed_update_id: i64,
    pub(super) journal_position: u64,
    pub(super) last_journal_recovery_at: i64,
    pub(super) selected_target: Option<SelectedTarget>,
    pub(super) agent_snapshot: Vec<AgentSnapshotEntry>,
    pub(super) processed_hook_signatures: Vec<String>,
    pub(super) pending: Option<PendingRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct SelectedTarget {
    pub(super) pane_id: String,
    pub(super) label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct AgentSnapshotEntry {
    pub(super) pane_id: String,
    pub(super) label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct PendingRequest {
    pub(super) request_id: String,
    pub(super) chat_id: String,
    pub(super) pane_id: String,
    #[serde(default)]
    pub(super) agent_kind: String,
    pub(super) target_label: String,
    pub(super) prompt_text: String,
    pub(super) prompt_hash: String,
    pub(super) sent_at: i64,
    #[serde(default)]
    pub(super) sent_at_ms: i64,
    pub(super) accepted_at: Option<i64>,
    #[serde(default)]
    pub(super) accepted_at_ms: Option<i64>,
    pub(super) last_status_at: Option<i64>,
    pub(super) draft_id: i64,
    pub(super) phase: String,
    #[serde(default)]
    pub(super) transcript_path: Option<String>,
    #[serde(default)]
    pub(super) approval_scan_offset: u64,
    #[serde(default)]
    pub(super) approval_call_id: Option<String>,
    #[serde(default)]
    pub(super) approval_justification: Option<String>,
}

pub(super) fn mark_update_processed(state: &mut TelegramState, update_id: i64) -> bool {
    if update_id <= state.last_processed_update_id {
        return false;
    }
    state.last_processed_update_id = update_id;
    state.update_offset = state.update_offset.max(update_id.saturating_add(1));
    true
}

pub(super) fn load_state() -> io::Result<TelegramState> {
    let path = crate::paths::telegram_state_path();
    match fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(TelegramState::default()),
        Err(err) => Err(err),
    }
}

pub(super) fn save_state(state: &TelegramState) -> io::Result<()> {
    let body = serde_json::to_string_pretty(state)?;
    fs::write(crate::paths::telegram_state_path(), body)
}

pub(super) fn journal_len() -> u64 {
    fs::metadata(crate::paths::hook_events_path())
        .map(|meta| meta.len())
        .unwrap_or(0)
}

pub(super) fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub(super) fn now_ms_i64() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
