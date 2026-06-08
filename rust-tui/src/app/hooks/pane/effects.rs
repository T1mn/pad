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
            app.trigger_codex_title_summary(request.session_id, request.turns, request.turn_count);
        }
    }
}
