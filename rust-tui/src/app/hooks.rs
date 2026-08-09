mod activity {
    use super::unix_now_ts;
    use crate::hook::HookEvent;
    use crate::log_debug;
    use crate::model::{AgentState, AgentType};
    use crate::sidebar::ThreadActivityOverride;

    pub(super) fn normalize_codex_rollout_paths_if_needed(event: &HookEvent) {
        let Some(path) = event.transcript_path.as_deref() else {
            return;
        };
        let pad_codex_home = crate::paths::pad_codex_home_dir()
            .to_string_lossy()
            .to_string();
        if !path.starts_with(&pad_codex_home) {
            return;
        }
        if let Err(err) = crate::codex_state::normalize_pad_codex_home_rollout_paths() {
            log_debug!("hook: codex rollout path normalization failed: {}", err);
        }
    }

    pub(super) fn app_thread_activity_from_hook(
        event: &HookEvent,
    ) -> Option<ThreadActivityOverride> {
        let working_dir = event.cwd.clone()?;
        let agent_type = infer_hook_agent_type(event)?;
        let updated_at = unix_now_ts();

        let (state, is_active) = match event.event.as_str() {
            "user_prompt_submit" => (AgentState::Busy, true),
            "stop" => (AgentState::Waiting, false),
            "session_start" => (AgentState::Idle, false),
            _ => (AgentState::Idle, false),
        };

        Some(ThreadActivityOverride {
            agent_type,
            session_id: event.session_id.clone(),
            transcript_path: event.transcript_path.clone(),
            working_dir,
            state,
            is_active,
            last_user_prompt: event.prompt.clone(),
            last_assistant_message: event.last_assistant_message.clone(),
            updated_at,
        })
    }

    fn infer_hook_agent_type(event: &HookEvent) -> Option<AgentType> {
        if let Some(path) = event.transcript_path.as_deref() {
            if path.contains("/.codex/") || path.contains("/.pad/codex-home/") {
                return Some(AgentType::Codex);
            }
            if path.contains("/.claude/") {
                return Some(AgentType::Claude);
            }
        }

        if let Some(session_id) = event.session_id.as_deref() {
            if crate::codex_state::thread_for_id(session_id)
                .ok()
                .flatten()
                .is_some()
            {
                return Some(AgentType::Codex);
            }
            if crate::claude_history::thread_for_id(session_id)
                .ok()
                .flatten()
                .is_some()
            {
                return Some(AgentType::Claude);
            }
            if crate::gemini_history::thread_for_id(session_id)
                .ok()
                .flatten()
                .is_some()
            {
                return Some(AgentType::Gemini);
            }
        }

        Some(AgentType::Codex)
    }
}
mod app_thread {
    use super::{
        activity, notification, unix_now_ts, App, APP_THREAD_ACTIVITY_MAX_ENTRIES,
        APP_THREAD_ACTIVITY_TTL_SECS,
    };
    use crate::app::state::FocusTarget;
    use crate::hook::HookEvent;
    use crate::notification_inbox::NotificationEntry;

    impl App {
        pub(super) fn apply_app_thread_hook_event(&mut self, event: HookEvent) {
            let Some(activity) = activity::app_thread_activity_from_hook(&event) else {
                return;
            };
            crate::session_continuity::record_hook_event(
                Some(&activity.agent_type),
                &event,
                activity.session_id.as_deref(),
                activity.transcript_path.as_deref(),
            );
            let pending_notification =
                notification::completion_notification_for_activity(&activity, &event);

            let selected_matches = self
                .selected_preview_thread()
                .map(|thread| {
                    thread.agent_type == activity.agent_type
                        && ((activity.session_id.is_some()
                            && activity.session_id == thread.session_id)
                            || (activity.transcript_path.is_some()
                                && activity.transcript_path == thread.transcript_path)
                            || thread.working_dir == activity.working_dir)
                })
                .unwrap_or(false);

            let key = activity
                .session_id
                .clone()
                .or(activity.transcript_path.clone())
                .unwrap_or_else(|| format!("{}:{}", activity.agent_type, activity.working_dir));
            self.sidebar.app_thread_activity.insert(key, activity);
            self.prune_app_thread_activity(unix_now_ts());
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
            if selected_matches {
                self.invalidate_preview();
            }
            self.dirty = true;

            if let Some(notification) = pending_notification {
                self.push_notification_entry(NotificationEntry::from_draft(
                    notification.draft,
                    unix_now_ts(),
                ));
                notification::emit_completion_notification(&self.config, notification.request);
            }
        }

        pub(crate) fn prune_app_thread_activity(&mut self, now_ts: i64) -> bool {
            let cutoff = now_ts.saturating_sub(APP_THREAD_ACTIVITY_TTL_SECS);
            let before = self.sidebar.app_thread_activity.len();
            self.sidebar
                .app_thread_activity
                .retain(|_, activity| activity.updated_at >= cutoff);

            if self.sidebar.app_thread_activity.len() > APP_THREAD_ACTIVITY_MAX_ENTRIES {
                let mut keys_by_freshness = self
                    .sidebar
                    .app_thread_activity
                    .iter()
                    .map(|(key, activity)| (key.clone(), activity.updated_at))
                    .collect::<Vec<_>>();
                keys_by_freshness
                    .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
                for key in keys_by_freshness
                    .iter()
                    .skip(APP_THREAD_ACTIVITY_MAX_ENTRIES)
                    .map(|item| &item.0)
                {
                    self.sidebar.app_thread_activity.remove(key);
                }
            }

            self.sidebar.app_thread_activity.len() != before
        }

        pub(super) fn panel_item_is_focused(&mut self, pane_id: &str) -> bool {
            !self.sidebar.show_tree
                && self.preview.focus == FocusTarget::Panel
                && self
                    .selected_preview_thread()
                    .and_then(|thread| thread.live_pane_id)
                    .map(|selected_pane_id| selected_pane_id == pane_id)
                    .unwrap_or(false)
        }

        pub fn clear_unread_stop_for_selected_panel(&mut self) {
            if self.sidebar.show_tree || self.preview.focus != FocusTarget::Panel {
                return;
            }

            let Some(selected_pane_id) = self
                .selected_preview_thread()
                .and_then(|thread| thread.live_pane_id)
            else {
                return;
            };

            if let Some(panel) = self
                .panels
                .iter_mut()
                .find(|panel| panel.pane_id == selected_pane_id)
            {
                panel.has_unread_stop = false;
            }
        }
    }
}
mod claude_history {
    use super::unix_now_ts;
    use crate::hook::HookEvent;
    use crate::model::{AgentPanel, AgentType};
    use std::path::PathBuf;

    pub(super) fn pane_claude_history_upsert_args(
        panel: &AgentPanel,
        event: &HookEvent,
        persisted_snapshot: Option<&crate::session_cache::SessionCacheSnapshot>,
    ) -> Option<(String, PathBuf, PathBuf, Option<String>, i64)> {
        if panel.agent_type != AgentType::Claude {
            return None;
        }

        let session_id = event
            .session_id
            .clone()
            .or_else(|| persisted_snapshot.map(|snapshot| snapshot.agent_session_id.clone()))
            .or_else(|| panel.agent_session_id.clone())?;

        let transcript_path = event
            .transcript_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| {
                persisted_snapshot
                    .and_then(|snapshot| snapshot.transcript_path.as_ref())
                    .map(PathBuf::from)
            })
            .or_else(|| panel.transcript_path.as_ref().map(PathBuf::from))?;

        let cwd = event
            .cwd
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(panel.working_dir.clone()));

        let title = event
            .prompt
            .clone()
            .or_else(|| persisted_snapshot.and_then(|snapshot| snapshot.last_user_prompt.clone()))
            .or_else(|| panel.last_user_prompt.clone());

        Some((session_id, transcript_path, cwd, title, unix_now_ts()))
    }
}
mod notification;
mod notification_text {
    use crate::model::AgentType;
    use crate::text_normalize::collapse_whitespace;
    use std::path::Path;

    pub(super) fn notification_agent_label(agent_type: &AgentType) -> &'static str {
        match agent_type {
            AgentType::Claude => "Claude",
            AgentType::Codex => "Codex",
            AgentType::Grok => "Grok",
            AgentType::Gemini => "Gemini",
            AgentType::OpenCode => "OpenCode",
            AgentType::Kimi => "Kimi",
            AgentType::Aider => "Aider",
            AgentType::Cursor => "Cursor",
            AgentType::Unknown => "Agent",
        }
    }

    pub(super) fn completion_notification_body(
        agent_type: &AgentType,
        session_id: Option<&str>,
        fallback_prompt: Option<&str>,
        working_dir: Option<&str>,
    ) -> String {
        fallback_prompt
            .map(normalize_notification_text)
            .or_else(|| lookup_notification_title(agent_type, session_id))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| notification_workdir_fallback(working_dir, session_id))
    }

    fn lookup_notification_title(
        agent_type: &AgentType,
        session_id: Option<&str>,
    ) -> Option<String> {
        let session_id = session_id?;
        match agent_type {
            AgentType::Codex => crate::codex_state::thread_for_id(session_id)
                .ok()
                .flatten()
                .and_then(|thread| thread.title.or(thread.first_user_message))
                .map(normalize_notification_text),
            AgentType::Claude => crate::claude_history::thread_for_id(session_id)
                .ok()
                .flatten()
                .and_then(|thread| thread.title)
                .map(normalize_notification_text),
            AgentType::Gemini => crate::gemini_history::thread_for_id(session_id)
                .ok()
                .flatten()
                .and_then(|thread| {
                    thread
                        .title
                        .or(thread.summary)
                        .or(thread.last_user_message)
                        .or(thread.first_user_message)
                })
                .map(normalize_notification_text),
            AgentType::Grok => crate::grok_history::thread_for_id(session_id)
                .ok()
                .flatten()
                .and_then(|thread| thread.title)
                .map(normalize_notification_text),
            _ => None,
        }
    }

    fn normalize_notification_text(text: impl AsRef<str>) -> String {
        truncate_notification_text(&collapse_whitespace(text.as_ref()), 72)
    }

    fn truncate_notification_text(text: &str, max_chars: usize) -> String {
        let mut truncated = String::new();
        for (idx, ch) in text.chars().enumerate() {
            if idx >= max_chars {
                truncated.push_str("...");
                return truncated;
            }
            truncated.push(ch);
        }
        truncated
    }

    fn notification_workdir_fallback(
        working_dir: Option<&str>,
        session_id: Option<&str>,
    ) -> String {
        working_dir
            .and_then(|path| Path::new(path).file_name())
            .and_then(|name| name.to_str())
            .map(normalize_notification_text)
            .filter(|name| !name.is_empty())
            .or_else(|| session_id.map(normalize_notification_text))
            .unwrap_or_else(|| "Session complete".to_string())
    }
}
mod pane;
mod title_summary {
    use crate::hook::HookEvent;
    use crate::model::{AgentPanel, AgentType};

    #[derive(Clone, Debug)]
    pub(super) struct PendingCodexTitleSummary {
        pub(super) session_id: String,
        pub(super) turns: Vec<crate::model::PreviewTurn>,
        pub(super) turn_count: usize,
    }

    pub(super) fn codex_title_summary_request_for_panel(
        codex_config: &crate::theme::CodexConfig,
        panel: &AgentPanel,
        event: &HookEvent,
        persisted_snapshot: Option<&crate::session_cache::SessionCacheSnapshot>,
    ) -> Option<PendingCodexTitleSummary> {
        if !crate::title_summary::is_enabled(codex_config)
            || event.event != "stop"
            || panel.agent_type != AgentType::Codex
        {
            return None;
        }

        let session_id = panel
            .agent_session_id
            .clone()
            .or_else(|| persisted_snapshot.map(|snapshot| snapshot.agent_session_id.clone()))
            .or_else(|| event.session_id.clone())?;

        let turns = persisted_snapshot
            .map(|snapshot| snapshot.recent_turns.to_vec())
            .unwrap_or_else(|| panel.cached_preview_turns.to_vec());
        let turn_count = turns
            .iter()
            .filter(|turn| !turn.question.trim().is_empty())
            .count();

        Some(PendingCodexTitleSummary {
            session_id,
            turns,
            turn_count,
        })
    }
}

use super::{unix_now_ts, App, APP_THREAD_ACTIVITY_MAX_ENTRIES, APP_THREAD_ACTIVITY_TTL_SECS};
use crate::hook::HookEvent;

impl App {
    pub fn apply_hook_event(&mut self, event: HookEvent) {
        activity::normalize_codex_rollout_paths_if_needed(&event);

        let Some(pane_id) = event.terminal.pane_id.clone() else {
            self.apply_app_thread_hook_event(event);
            return;
        };

        self.apply_pane_hook_event(event, pane_id);
    }
}

#[cfg(test)]
pub(crate) mod hooks_tests;
