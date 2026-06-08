use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Default, Serialize)]
pub(in crate::chat::providers::telegram) struct TelegramState {
    pub(in crate::chat::providers::telegram) update_offset: i64,
    pub(in crate::chat::providers::telegram) last_processed_update_id: i64,
    pub(in crate::chat::providers::telegram) journal_position: u64,
    pub(in crate::chat::providers::telegram) last_journal_recovery_at: i64,
    pub(in crate::chat::providers::telegram) selected_target: Option<SelectedTarget>,
    pub(in crate::chat::providers::telegram) agent_snapshot: Vec<AgentSnapshotEntry>,
    pub(in crate::chat::providers::telegram) processed_hook_signatures: Vec<String>,
    pub(in crate::chat::providers::telegram) pending_requests: Vec<PendingRequest>,
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
pub(in crate::chat::providers::telegram) struct SelectedTarget {
    pub(in crate::chat::providers::telegram) pane_id: String,
    pub(in crate::chat::providers::telegram) label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::chat::providers::telegram) struct AgentSnapshotEntry {
    pub(in crate::chat::providers::telegram) pane_id: String,
    pub(in crate::chat::providers::telegram) label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::chat::providers::telegram) struct PendingRequest {
    pub(in crate::chat::providers::telegram) request_id: String,
    pub(in crate::chat::providers::telegram) chat_id: String,
    pub(in crate::chat::providers::telegram) pane_id: String,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) agent_kind: String,
    pub(in crate::chat::providers::telegram) target_label: String,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) session_id: Option<String>,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) working_dir: String,
    pub(in crate::chat::providers::telegram) prompt_text: String,
    pub(in crate::chat::providers::telegram) prompt_hash: String,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) turn_id: Option<String>,
    pub(in crate::chat::providers::telegram) sent_at: i64,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) sent_at_ms: i64,
    pub(in crate::chat::providers::telegram) accepted_at: Option<i64>,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) accepted_at_ms: Option<i64>,
    pub(in crate::chat::providers::telegram) last_status_at: Option<i64>,
    pub(in crate::chat::providers::telegram) draft_id: i64,
    pub(in crate::chat::providers::telegram) phase: String,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) transcript_path: Option<String>,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) result_scan_offset: u64,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) failure_scan_offset: u64,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) last_failure_check_at: Option<i64>,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) approval_scan_offset: u64,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) approval_call_id: Option<String>,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) approval_justification: Option<String>,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) completed_text: Option<String>,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) completed_source: Option<String>,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) delivery_attempts: u32,
    #[serde(default)]
    pub(in crate::chat::providers::telegram) delivery_retry_at: i64,
}
