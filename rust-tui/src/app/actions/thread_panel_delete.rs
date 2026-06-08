use super::helpers::{delete_failed_title, delete_hide_failed_title};
use super::*;

impl App {
    pub fn delete_panel(&mut self, panel: &crate::model::AgentPanel) {
        self.sidebar.pending_sidebar_selection_index = self.table_state.selected();
        let target_thread = self
            .selected_preview_thread()
            .filter(|thread| thread.live_pane_id.as_deref() == Some(panel.pane_id.as_str()));

        log_debug!(
            "delete_panel: pane_id={} target_thread={} session_id={:?} agent_type={:?}",
            panel.pane_id,
            target_thread
                .as_ref()
                .map(|thread| thread.key.as_str())
                .unwrap_or("-"),
            target_thread
                .as_ref()
                .and_then(|thread| thread.session_id.as_deref()),
            target_thread.as_ref().map(|thread| &thread.agent_type),
        );

        let kill_result = std::process::Command::new("tmux")
            .args(["kill-pane", "-t", &panel.pane_id])
            .output();

        match kill_result {
            Ok(output) if output.status.success() => {
                self.apply_deleted_panel_locally(&panel.pane_id);
                if let Some(thread) = target_thread.as_ref() {
                    let should_soft_delete = thread
                        .session_id
                        .as_deref()
                        .map(|session_id| {
                            !self.panels.iter().any(|candidate| {
                                candidate.pane_id != panel.pane_id
                                    && candidate.agent_type == thread.agent_type
                                    && candidate.agent_session_id.as_deref() == Some(session_id)
                            })
                        })
                        .unwrap_or(false);

                    if should_soft_delete {
                        if let Some(session_id) = thread.session_id.as_deref() {
                            if let Err(err) = crate::thread_meta::set_thread_deleted(
                                &thread.agent_type.to_string(),
                                session_id,
                                true,
                            ) {
                                self.show_action_toast(
                                    delete_hide_failed_title(self.locale),
                                    &err.to_string(),
                                );
                            }
                        }
                    }
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let message = if stderr.is_empty() {
                    format!("tmux kill-pane exited with {}", output.status)
                } else {
                    stderr
                };
                self.show_action_toast(delete_failed_title(self.locale), &message);
            }
            Err(err) => {
                self.show_action_toast(delete_failed_title(self.locale), &err.to_string());
            }
        }

        self.refresh_panels();
    }

    pub(crate) fn apply_deleted_panel_locally(&mut self, pane_id: &str) {
        let original_len = self.panels.len();
        self.panels.retain(|panel| panel.pane_id != pane_id);
        if self.panels.len() == original_len {
            return;
        }

        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        if self.selected_panel().is_none() {
            self.focus_panel();
        }
        self.invalidate_preview();
        self.dirty = true;
    }

    pub fn refresh_panels(&mut self) {
        if !self.scan_in_progress {
            self.trigger_async_scan();
        }
    }
}
