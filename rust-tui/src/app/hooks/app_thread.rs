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
                    && ((activity.session_id.is_some() && activity.session_id == thread.session_id)
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
