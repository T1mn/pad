mod effects {
    use super::super::{claude_history, notification, title_summary, unix_now_ts, App};
    use crate::hook::HookEvent;
    use crate::model::AgentPanel;
    use crate::notification_inbox::NotificationEntry;
    use crate::session_cache::SessionCacheSnapshot;
    use std::path::PathBuf;

    type ClaudeHistoryUpsert = (String, PathBuf, PathBuf, Option<String>, i64);

    pub(super) struct PendingPaneHookEffects {
        claude_history_upsert: Option<ClaudeHistoryUpsert>,
        notification: Option<notification::PendingNotification>,
        title_summary: Option<title_summary::PendingCodexTitleSummary>,
    }

    impl PendingPaneHookEffects {
        pub(super) fn from_panel(
            codex_config: &crate::theme::CodexConfig,
            panel: &AgentPanel,
            event: &HookEvent,
            persisted_snapshot: Option<&SessionCacheSnapshot>,
        ) -> Self {
            Self {
                claude_history_upsert: claude_history::pane_claude_history_upsert_args(
                    panel,
                    event,
                    persisted_snapshot,
                ),
                notification: notification::completion_notification_for_panel(
                    panel,
                    event,
                    persisted_snapshot,
                ),
                title_summary: title_summary::codex_title_summary_request_for_panel(
                    codex_config,
                    panel,
                    event,
                    persisted_snapshot,
                ),
            }
        }

        pub(super) fn apply(self, app: &mut App) {
            if let Some((session_id, transcript_path, cwd, title, updated_at)) =
                self.claude_history_upsert
            {
                if let Err(err) = crate::claude_history::upsert_hook_session(
                    &session_id,
                    &transcript_path,
                    &cwd,
                    title.as_deref(),
                    updated_at,
                ) {
                    crate::log_debug!("claude_history: pane hook upsert failed: {}", err);
                }
            }

            if let Some(notification) = self.notification {
                app.push_notification_entry(NotificationEntry::from_draft(
                    notification.draft,
                    unix_now_ts(),
                ));
                notification::emit_completion_notification(&app.config, notification.request);
            }

            if let Some(request) = self.title_summary {
                app.trigger_codex_title_summary(
                    request.session_id,
                    request.turns,
                    request.turn_count,
                );
            }
        }
    }
}
mod panel_update {
    use crate::hook::HookEvent;
    use crate::log_debug;
    use crate::model::{AgentPanel, AgentState, SessionCacheState};
    use crate::session_cache::SessionCacheSnapshot;

    pub(super) fn apply_panel_hook_event(
        panel: &mut AgentPanel,
        event: &HookEvent,
        panel_item_focused: bool,
    ) -> Option<SessionCacheSnapshot> {
        let persisted_snapshot = persist_hook_event(panel, event);
        apply_event_metadata(panel, event);
        apply_event_state(panel, event, panel_item_focused);
        crate::session_continuity::record_hook_event(
            Some(&panel.agent_type),
            event,
            panel.agent_session_id.as_deref(),
            panel.transcript_path.as_deref(),
        );

        if let Some(snapshot) = persisted_snapshot.as_ref() {
            apply_persisted_snapshot(panel, snapshot);
        }
        persisted_snapshot
    }

    fn apply_event_metadata(panel: &mut AgentPanel, event: &HookEvent) {
        if event.session_id.is_some() {
            panel.agent_session_id = event.session_id.clone();
        }
        if event.transcript_path.is_some() {
            panel.transcript_path = event.transcript_path.clone();
        }
    }

    fn apply_event_state(panel: &mut AgentPanel, event: &HookEvent, panel_item_focused: bool) {
        match event.event.as_str() {
            "session_start" => {}
            "user_prompt_submit" => {
                panel.state = AgentState::Busy;
                panel.is_active = true;
                panel.last_user_prompt = event.prompt.clone();
                panel.last_assistant_message = None;
                panel.has_unread_stop = false;
            }
            "stop" => {
                panel.state = AgentState::Waiting;
                panel.is_active = false;
                panel.has_unread_stop = !panel_item_focused;
                if event.last_assistant_message.is_some() {
                    panel.last_assistant_message = event.last_assistant_message.clone();
                }
            }
            _ => {}
        }
    }

    fn persist_hook_event(panel: &AgentPanel, event: &HookEvent) -> Option<SessionCacheSnapshot> {
        match crate::session_cache::persist_hook_event(panel, event) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                log_debug!("session_cache: persist hook failed: {}", err);
                None
            }
        }
    }

    fn apply_persisted_snapshot(panel: &mut AgentPanel, snapshot: &SessionCacheSnapshot) {
        panel.agent_session_id = Some(snapshot.agent_session_id.clone());
        panel.transcript_path = snapshot.transcript_path.clone();
        panel.cached_preview_turns = snapshot.recent_turns.clone();
        panel.last_user_prompt = snapshot.last_user_prompt.clone();
        panel.last_assistant_message = snapshot.last_assistant_message.clone();
        panel.session_cache_state = Some(SessionCacheState::Confirmed);
    }
}
mod subagent {
    use crate::hook::HookEvent;
    use crate::log_debug;
    use crate::model::{AgentPanel, AgentType};

    pub(super) fn handle_codex_subagent_event(
        panel: &mut AgentPanel,
        event: &HookEvent,
        pane_id: &str,
    ) -> bool {
        if panel.agent_type != AgentType::Codex {
            return false;
        }

        let Some(subagent_session_id) = event.session_id.as_deref() else {
            return false;
        };
        let Ok(Some(parent_thread_id)) =
            crate::codex_state::subagent_parent_thread_id(subagent_session_id)
        else {
            return false;
        };

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
        true
    }
}

use super::App;
use crate::hook::HookEvent;

impl App {
    pub(super) fn apply_pane_hook_event(&mut self, event: HookEvent, pane_id: String) {
        let panel_item_focused = self.panel_item_is_focused(&pane_id);
        let should_refresh_preview = self
            .selected_panel()
            .map(|panel| panel.pane_id == pane_id)
            .unwrap_or(false);

        let mut pending_effects = None;

        if let Some(panel) = self.panels.iter_mut().find(|p| p.pane_id == pane_id) {
            if subagent::handle_codex_subagent_event(panel, &event, &pane_id) {
                self.invalidate_sidebar_cache();
                if should_refresh_preview {
                    self.invalidate_preview();
                }
                self.dirty = true;
                return;
            }

            let persisted_snapshot =
                panel_update::apply_panel_hook_event(panel, &event, panel_item_focused);
            pending_effects = Some(effects::PendingPaneHookEffects::from_panel(
                &self.config.codex,
                panel,
                &event,
                persisted_snapshot.as_ref(),
            ));

            self.invalidate_sidebar_cache();
            if should_refresh_preview {
                self.invalidate_preview();
            }
            self.dirty = true;
        }

        if let Some(effects) = pending_effects {
            effects.apply(self);
        }
    }
}
