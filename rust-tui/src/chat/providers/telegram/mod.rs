mod api;
mod callbacks;
mod commands;
mod daemon;
mod help;
mod hooks;
mod locale;
mod pending;
mod render;
mod state;

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
use crate::tmux_dispatch;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::net::{UnixListener, UnixStream};
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
#[allow(unused_imports)]
pub use self::daemon::{
    daemon_is_running, ensure_daemon_running, ensure_embedded_daemon_running, restart_daemon,
    run_daemon, stop_daemon, sync_daemon,
};
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
