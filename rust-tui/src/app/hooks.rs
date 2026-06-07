mod activity;
mod app_thread;
mod claude_history;
mod notification;
mod notification_text;
mod title_summary;

use super::{unix_now_ts, App, APP_THREAD_ACTIVITY_MAX_ENTRIES, APP_THREAD_ACTIVITY_TTL_SECS};
use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{AgentState, AgentStateSource, AgentType, SessionCacheState};
use crate::notification_inbox::NotificationEntry;

impl App {
    pub fn apply_hook_event(&mut self, event: HookEvent) {
        activity::normalize_codex_rollout_paths_if_needed(&event);

        let pane_id = match event.tmux.pane_id.clone() {
            Some(id) => id,
            None => {
                self.apply_app_thread_hook_event(event);
                return;
            }
        };
        let panel_item_focused = self.panel_item_is_focused(&pane_id);

        let should_refresh_preview = self
            .selected_panel()
            .map(|panel| panel.pane_id == pane_id)
            .unwrap_or(false);

        let mut persisted_snapshot = None;
        let mut pending_claude_history_upsert = None;
        let mut pending_notification = None;
        let mut pending_title_summary = None;

        if let Some(panel) = self.panels.iter_mut().find(|p| p.pane_id == pane_id) {
            if panel.agent_type == AgentType::Codex {
                if let Some(subagent_session_id) = event.session_id.as_deref() {
                    if let Ok(Some(parent_thread_id)) =
                        crate::codex_state::subagent_parent_thread_id(subagent_session_id)
                    {
                        if panel.agent_session_id.is_none()
                            || panel.agent_session_id.as_deref() == Some(subagent_session_id)
                        {
                            panel.agent_session_id = Some(parent_thread_id.clone());
                        }
                        log_debug!(
                            "hook: ignoring codex subagent event pane={} subagent_session={} parent_session={}",
                            pane_id,
                            subagent_session_id,
                            parent_thread_id
                        );
                        self.invalidate_sidebar_cache();
                        if should_refresh_preview {
                            self.invalidate_preview();
                        }
                        self.dirty = true;
                        return;
                    }
                }
            }

            if event.session_id.is_some() {
                panel.agent_session_id = event.session_id.clone();
            }
            if event.transcript_path.is_some() {
                panel.transcript_path = event.transcript_path.clone();
            }
            match event.event.as_str() {
                "session_start" => {}
                "user_prompt_submit" => {
                    panel.state = AgentState::Busy;
                    panel.state_source = AgentStateSource::Hook;
                    panel.is_active = true;
                    panel.last_user_prompt = event.prompt.clone();
                    panel.last_assistant_message = None;
                    panel.has_unread_stop = false;
                }
                "stop" => {
                    panel.state = AgentState::Waiting;
                    panel.state_source = AgentStateSource::Hook;
                    panel.is_active = false;
                    panel.has_unread_stop = !panel_item_focused;
                    if event.last_assistant_message.is_some() {
                        panel.last_assistant_message = event.last_assistant_message.clone();
                    }
                }
                _ => {}
            }

            crate::session_continuity::record_hook_event(
                Some(&panel.agent_type),
                &event,
                panel.agent_session_id.as_deref(),
                panel.transcript_path.as_deref(),
            );

            match crate::session_cache::persist_hook_event(panel, &event) {
                Ok(snapshot) => persisted_snapshot = snapshot,
                Err(err) => log_debug!("session_cache: persist hook failed: {}", err),
            }

            if let Some(snapshot) = persisted_snapshot.as_ref() {
                panel.agent_session_id = Some(snapshot.agent_session_id.clone());
                panel.transcript_path = snapshot.transcript_path.clone();
                panel.cached_preview_turns = snapshot.recent_turns.clone();
                panel.last_user_prompt = snapshot.last_user_prompt.clone();
                panel.last_assistant_message = snapshot.last_assistant_message.clone();
                panel.session_cache_state = Some(SessionCacheState::Confirmed);
            }

            pending_claude_history_upsert = claude_history::pane_claude_history_upsert_args(
                panel,
                &event,
                persisted_snapshot.as_ref(),
            );
            pending_notification = notification::completion_notification_for_panel(
                panel,
                &event,
                persisted_snapshot.as_ref(),
            );
            pending_title_summary = title_summary::codex_title_summary_request_for_panel(
                &self.config.codex,
                panel,
                &event,
                persisted_snapshot.as_ref(),
            );

            self.invalidate_sidebar_cache();
            if should_refresh_preview {
                self.invalidate_preview();
            }
            self.dirty = true;
        }

        if let Some((session_id, transcript_path, cwd, title, updated_at)) =
            pending_claude_history_upsert
        {
            if let Err(err) = crate::claude_history::upsert_hook_session(
                &session_id,
                &transcript_path,
                &cwd,
                title.as_deref(),
                updated_at,
            ) {
                log_debug!("claude_history: pane hook upsert failed: {}", err);
            }
        }

        if let Some(notification) = pending_notification {
            self.push_notification_entry(NotificationEntry::from_draft(
                notification.draft,
                unix_now_ts(),
            ));
            notification::emit_completion_notification(&self.config, notification.request);
        }

        if let Some(request) = pending_title_summary {
            self.trigger_codex_title_summary(request.session_id, request.turns, request.turn_count);
        }
    }
}

#[cfg(test)]
mod hooks_tests;
