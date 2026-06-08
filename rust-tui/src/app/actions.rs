mod codex_restart;
mod helpers;
mod notification_inbox;
mod opencode_attach;
mod opencode_cli;
mod opencode_diagnostics;
mod opencode_export;
mod opencode_github;
mod opencode_import;
mod opencode_plugin;
mod opencode_pr;
mod opencode_run;
mod opencode_serve;
mod opencode_stats;
mod opencode_web;
mod relay_reload;
mod settings;
mod thread_actions;
mod thread_meta_edit;
mod thread_panel_delete;
mod tree;

use super::state::{Mode, SettingsDetailKind, SettingsFocus};
use super::{App, ThreadActionKind, ThreadMetaEditKind};
use crate::i18n::Locale;
use crate::log_debug;
use crate::model::AgentType;
use crate::sidebar::{SidebarItem, SidebarThread};

pub(crate) use helpers::settings_item_search_blob;

#[cfg(test)]
mod tests;
