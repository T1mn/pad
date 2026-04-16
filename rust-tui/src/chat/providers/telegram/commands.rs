use super::*;

mod command;
mod diag;
mod help_actions;
mod history;
mod plain;
mod restart;
mod slash;
mod update;

const PAD_DEFAULT_SESSION_NAME: &str = "pad";
const PAD_CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PadRestartTarget {
    RespawnPane(String),
    NewDetachedSession(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PadRestartPlan {
    pub(super) target: PadRestartTarget,
    pub(super) start_dir: String,
    pub(super) shell_command: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionDiagContext {
    target_label: String,
    pane_id: Option<String>,
    request_id: Option<String>,
    session_id: Option<String>,
    transcript_path: Option<String>,
    continuity: Option<crate::session_continuity::ContinuitySnapshot>,
}

use command::handle_command;
use diag::send_session_diag;
use help_actions::send_help_message;
use history::send_recent_history;
use plain::handle_plain_text;
use slash::dispatch_codex_slash_command;

pub(super) use diag::send_pad_status_report;
pub(super) use help_actions::{edit_help_message, send_agent_list};
pub(super) use update::handle_update;

#[cfg(test)]
pub(super) use diag::build_pad_status_body;
#[cfg(test)]
pub(super) use history::{format_recent_history_message, recent_history_turns};
#[cfg(test)]
pub(super) use restart::{build_pad_restart_shell_command, select_pad_restart_target};
