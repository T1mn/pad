const TELEGRAM_TIMEOUT_SECS: u64 = 12;
const TELEGRAM_POLL_TIMEOUT_SECS: u64 = 1;
const TELEGRAM_MAX_TEXT_LEN: usize = 3500;
const TELEGRAM_INTERACTIVE_TIMEOUT_SECS: u64 = 4;
const TELEGRAM_STATUS_TIMEOUT_SECS: u64 = 3;

mod chat_id;
mod client;
mod commands;
mod interactive;
mod messages;
mod text;
mod types;
mod updates;

pub(super) use chat_id::telegram_chat_id_value;
pub(super) use commands::set_my_commands;
pub(super) use interactive::{answer_callback_query, send_chat_action, send_message_draft};
pub(super) use messages::{edit_message, send_message, send_text};
#[cfg(test)]
pub(super) use text::chunk_text;
pub(super) use types::{TelegramCallbackQuery, TelegramUpdate};
pub(super) use updates::{fetch_me, get_updates};
