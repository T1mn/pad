mod api;
mod callbacks;
mod commands;
mod daemon;
mod help;
mod hooks;
mod locale;
mod native_terminal {
    use crate::socket_api::model::ApiRequest;
    use std::io;

    pub(super) fn dispatch_prompt(pane_id: &str, prompt: &str) -> io::Result<()> {
        send(ApiRequest {
            action: "prompt".to_string(),
            pane_id: Some(pane_id.to_string()),
            prompt: Some(prompt.to_string()),
            ..ApiRequest::default()
        })
        .map(|_| ())
    }

    pub(super) fn capture_pane_tail(pane_id: &str, lines: usize) -> io::Result<String> {
        let response = send(ApiRequest {
            action: "capture".to_string(),
            pane_id: Some(pane_id.to_string()),
            ..ApiRequest::default()
        })?;
        let content = response
            .data
            .and_then(|data| data.get("content").cloned())
            .and_then(|content| content.as_str().map(str::to_string))
            .unwrap_or_default();
        Ok(content
            .lines()
            .rev()
            .take(lines)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n"))
    }

    pub(super) fn send_escape(pane_id: &str) -> io::Result<()> {
        send(ApiRequest {
            action: "escape".to_string(),
            pane_id: Some(pane_id.to_string()),
            ..ApiRequest::default()
        })
        .map(|_| ())
    }

    pub(super) fn send_approval_key(pane_id: &str, key: &str) -> io::Result<()> {
        send(ApiRequest {
            action: "approval".to_string(),
            pane_id: Some(pane_id.to_string()),
            command: Some(key.to_string()),
            ..ApiRequest::default()
        })
        .map(|_| ())
    }

    fn send(request: ApiRequest) -> io::Result<crate::socket_api::model::ApiResponse> {
        let response = crate::socket_api::client::send_request(&request)?;
        if response.ok {
            Ok(response)
        } else {
            Err(io::Error::other(response.message))
        }
    }
}
mod pending;
mod render;
mod state {
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
    mod model {
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
                        existing.request_id == pending.request_id
                            || existing.pane_id == pending.pane_id
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
    }
    mod pending {
        use super::model::{PendingRequest, TelegramState};

        pub(in crate::chat::providers::telegram) fn mark_update_processed(
            state: &mut TelegramState,
            update_id: i64,
        ) -> bool {
            if update_id <= state.last_processed_update_id {
                return false;
            }
            state.last_processed_update_id = update_id;
            state.update_offset = state.update_offset.max(update_id.saturating_add(1));
            true
        }

        pub(in crate::chat::providers::telegram) fn pending_request_index_by_id(
            state: &TelegramState,
            request_id: &str,
        ) -> Option<usize> {
            state
                .pending_requests
                .iter()
                .position(|pending| pending.request_id == request_id)
        }

        pub(in crate::chat::providers::telegram) fn pending_request_index_by_pane(
            state: &TelegramState,
            pane_id: &str,
        ) -> Option<usize> {
            state
                .pending_requests
                .iter()
                .position(|pending| pending.pane_id == pane_id)
        }

        pub(in crate::chat::providers::telegram) fn remove_pending_request(
            state: &mut TelegramState,
            request_id: &str,
        ) -> Option<PendingRequest> {
            let index = pending_request_index_by_id(state, request_id)?;
            Some(state.pending_requests.remove(index))
        }

        pub(in crate::chat::providers::telegram) fn remove_selected_target_pending_request(
            state: &mut TelegramState,
        ) -> Option<PendingRequest> {
            let pane_id = state.selected_target.as_ref()?.pane_id.clone();
            let index = pending_request_index_by_pane(state, &pane_id)?;
            Some(state.pending_requests.remove(index))
        }
    }
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
}

#[cfg(test)]
use self::api::chunk_text;
use self::api::{
    answer_callback_query, edit_message, fetch_me, get_updates, send_chat_action, send_message,
    send_message_draft, send_text, set_my_commands, telegram_chat_id_value, TelegramCallbackQuery,
    TelegramUpdate,
};
#[cfg(test)]
use self::callbacks::{approval_callback_data, parse_approval_callback_data};
#[cfg(test)]
use self::help::{build_help_keyboard, help_page_html};
use self::help::{help_message_payload, HelpPage};
use self::locale::{locale_prefers_chinese, telegram_locale, tg, tg_fmt, tg_fmt2, tg_fmt3};
use self::native_terminal as terminal_remote;
use self::state::{
    journal_len, load_state, mark_update_processed, next_draft_id, next_request_id, now_ms_i64,
    now_ts, pending_request_index_by_id, pending_request_index_by_pane, remove_pending_request,
    remove_selected_target_pending_request, save_state, AgentSnapshotEntry, PendingRequest,
    SelectedTarget, TelegramState,
};
use crate::chat::approval::{scan_codex_approval_updates, transcript_len, CodexApprovalRequest};
use crate::chat::backend::{
    build_slash_command_text, compact_target_label, invalidate_live_panels, live_panels,
    pad_is_online, panel_display_title, summarize_pane_capture,
};
use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{AgentPanel, AgentState, AgentType};
use crate::runtime_status;
use crate::sound::SoundEvent;
use crate::theme::Config;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::sleep;

type TelegramError = Box<dyn std::error::Error + Send + Sync>;
type TelegramResult<T> = Result<T, TelegramError>;

fn telegram_error(err: impl std::fmt::Display) -> TelegramError {
    io::Error::other(err.to_string()).into()
}

fn play_sound_event(config: &Config, event: SoundEvent) {
    if let Err(err) = crate::sound::play_event(&config.sound, event) {
        log_debug!(
            "telegram: sound playback failed event={:?} err={}",
            event,
            err
        );
    }
}

use self::callbacks::{handle_callback_query, send_codex_approval_prompt};
use self::commands::{edit_help_message, handle_update, send_agent_list, send_pad_status_report};
pub use self::daemon::{ensure_embedded_daemon_running, restart_daemon, run_daemon, sync_daemon};
#[cfg(test)]
use self::hooks::should_probe_hook_journal_inner;
use self::hooks::{
    apply_hook_event_to_pending, daemon_socket_is_active, remember_processed_hook_event,
    should_probe_hook_journal, start_direct_hook_listener,
};
#[cfg(test)]
use self::pending::pending_status_text;
use self::pending::{
    deliver_pending_result, finalize_pending_feedback, pending_accepted_ms, pending_sent_ms,
    pending_status_summary_line, process_codex_pending_approval, process_hook_journal,
    process_pending_result_delivery, process_pending_rollout_failures, process_pending_timeout,
    refresh_pending_feedback, DraftFeedbackGate,
};
use self::render::{
    build_agent_keyboard, format_agent_line, format_agent_line_for_button, truncate_chars,
    truncate_for_log,
};

const PENDING_TIMEOUT_SECS: i64 = 2 * 60 * 60;
const PENDING_FAILURE_SCAN_DELAY_SECS: i64 = 30;
const PENDING_FAILURE_SCAN_INTERVAL_SECS: i64 = 5;
const JOURNAL_RECOVERY_RETRY_SECS: i64 = 3;
const JOURNAL_RECOVERY_STALL_SECS: i64 = 5;
const RESULT_DELIVERY_RETRY_SECS: i64 = 5;
const SLASH_POLL_INTERVAL_MS: u64 = 90;
static RECENT_HOOK_SIGNATURES: LazyLock<Mutex<Vec<String>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static DRAFT_FEEDBACK_GATES: LazyLock<Mutex<HashMap<i64, Arc<DraftFeedbackGate>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
mod tests;
