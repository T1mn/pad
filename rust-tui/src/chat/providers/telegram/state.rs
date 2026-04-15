use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, Serialize)]
pub(super) struct TelegramState {
    pub(super) update_offset: i64,
    pub(super) last_processed_update_id: i64,
    pub(super) journal_position: u64,
    pub(super) last_journal_recovery_at: i64,
    pub(super) selected_target: Option<SelectedTarget>,
    pub(super) agent_snapshot: Vec<AgentSnapshotEntry>,
    pub(super) processed_hook_signatures: Vec<String>,
    pub(super) pending_requests: Vec<PendingRequest>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct TelegramStateDisk {
    update_offset: i64,
    last_processed_update_id: i64,
    journal_position: u64,
    last_journal_recovery_at: i64,
    selected_target: Option<SelectedTarget>,
    agent_snapshot: Vec<AgentSnapshotEntry>,
    processed_hook_signatures: Vec<String>,
    pending_requests: Vec<PendingRequest>,
    pending: Option<PendingRequest>,
}

impl From<TelegramStateDisk> for TelegramState {
    fn from(disk: TelegramStateDisk) -> Self {
        let mut pending_requests = disk.pending_requests;
        if let Some(pending) = disk.pending {
            let duplicate = pending_requests.iter().any(|existing| {
                existing.request_id == pending.request_id || existing.pane_id == pending.pane_id
            });
            if !duplicate {
                pending_requests.push(pending);
            }
        }
        Self {
            update_offset: disk.update_offset,
            last_processed_update_id: disk.last_processed_update_id,
            journal_position: disk.journal_position,
            last_journal_recovery_at: disk.last_journal_recovery_at,
            selected_target: disk.selected_target,
            agent_snapshot: disk.agent_snapshot,
            processed_hook_signatures: disk.processed_hook_signatures,
            pending_requests,
        }
    }
}

impl<'de> Deserialize<'de> for TelegramState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        TelegramStateDisk::deserialize(deserializer).map(Into::into)
    }
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
    #[serde(default)]
    pub(super) session_id: Option<String>,
    #[serde(default)]
    pub(super) working_dir: String,
    pub(super) prompt_text: String,
    pub(super) prompt_hash: String,
    #[serde(default)]
    pub(super) turn_id: Option<String>,
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
    pub(super) result_scan_offset: u64,
    #[serde(default)]
    pub(super) approval_scan_offset: u64,
    #[serde(default)]
    pub(super) approval_call_id: Option<String>,
    #[serde(default)]
    pub(super) approval_justification: Option<String>,
    #[serde(default)]
    pub(super) completed_text: Option<String>,
    #[serde(default)]
    pub(super) completed_source: Option<String>,
    #[serde(default)]
    pub(super) delivery_attempts: u32,
    #[serde(default)]
    pub(super) delivery_retry_at: i64,
}

pub(super) fn mark_update_processed(state: &mut TelegramState, update_id: i64) -> bool {
    if update_id <= state.last_processed_update_id {
        return false;
    }
    state.last_processed_update_id = update_id;
    state.update_offset = state.update_offset.max(update_id.saturating_add(1));
    true
}

static NEXT_REQUEST_ID: LazyLock<AtomicU64> =
    LazyLock::new(|| AtomicU64::new((now_ms_i64().max(1) as u64).saturating_mul(1000)));
static NEXT_DRAFT_ID: LazyLock<AtomicU64> =
    LazyLock::new(|| AtomicU64::new((now_ms_i64().max(1) as u64).saturating_mul(1000)));

pub(super) fn next_request_id() -> String {
    format!("tg-{}", NEXT_REQUEST_ID.fetch_add(1, Ordering::SeqCst))
}

pub(super) fn next_draft_id() -> i64 {
    NEXT_DRAFT_ID.fetch_add(1, Ordering::SeqCst) as i64
}

pub(super) fn pending_request_index_by_id(
    state: &TelegramState,
    request_id: &str,
) -> Option<usize> {
    state
        .pending_requests
        .iter()
        .position(|pending| pending.request_id == request_id)
}

pub(super) fn pending_request_index_by_pane(state: &TelegramState, pane_id: &str) -> Option<usize> {
    state
        .pending_requests
        .iter()
        .position(|pending| pending.pane_id == pane_id)
}

pub(super) fn remove_pending_request(
    state: &mut TelegramState,
    request_id: &str,
) -> Option<PendingRequest> {
    let index = pending_request_index_by_id(state, request_id)?;
    Some(state.pending_requests.remove(index))
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
