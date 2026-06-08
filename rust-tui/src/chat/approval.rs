mod answers;
mod failures;
mod requests;

use std::fs;

pub(super) use answers::scan_codex_answer_updates;
pub(super) use failures::scan_codex_failure_updates;
pub(super) use requests::scan_codex_approval_updates;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CodexApprovalRequest {
    pub(super) call_id: String,
    pub(super) justification: String,
}

pub(super) struct CodexApprovalScanResult {
    pub(super) active_request: Option<CodexApprovalRequest>,
    pub(super) next_offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CodexFailureEvent {
    pub(super) message: String,
    pub(super) error_info: Option<String>,
}

pub(super) struct CodexFailureScanResult {
    pub(super) failure: Option<CodexFailureEvent>,
    pub(super) next_offset: u64,
}

pub(super) fn transcript_len(path: &str) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}
