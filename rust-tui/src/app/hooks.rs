mod activity;
mod app_thread;
mod claude_history;
mod notification;
mod notification_text;
mod pane;
mod title_summary;

use super::{unix_now_ts, App, APP_THREAD_ACTIVITY_MAX_ENTRIES, APP_THREAD_ACTIVITY_TTL_SECS};
use crate::hook::HookEvent;

impl App {
    pub fn apply_hook_event(&mut self, event: HookEvent) {
        activity::normalize_codex_rollout_paths_if_needed(&event);

        let Some(pane_id) = event.tmux.pane_id.clone() else {
            self.apply_app_thread_hook_event(event);
            return;
        };

        self.apply_pane_hook_event(event, pane_id);
    }
}

#[cfg(test)]
mod hooks_tests;
