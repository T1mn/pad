use super::super::super::App;
use crate::model::AgentPanel;

impl App {
    pub(in crate::app::async_ops::scan) fn panels_affecting_refresh_changed(
        &self,
        next_panels: &[AgentPanel],
    ) -> bool {
        if self.panels.len() != next_panels.len() {
            return true;
        }

        for next in next_panels {
            let Some(current) = self
                .panels
                .iter()
                .find(|panel| panel.pane_id == next.pane_id)
            else {
                return true;
            };
            if current.session != next.session
                || current.window != next.window
                || current.window_index != next.window_index
                || current.pane != next.pane
                || current.working_dir != next.working_dir
                || current.agent_type != next.agent_type
                || current.state != next.state
                || current.state_source != next.state_source
                || current.is_active != next.is_active
                || current.transcript_path != next.transcript_path
                || current.cached_preview_turns != next.cached_preview_turns
                || current.session_cache_state != next.session_cache_state
                || current.git_info != next.git_info
                || current.pid != next.pid
                || current.last_user_prompt != next.last_user_prompt
                || current.last_assistant_message != next.last_assistant_message
                || current.agent_session_id != next.agent_session_id
                || current.has_unread_stop != next.has_unread_stop
            {
                return true;
            }
        }

        false
    }
}
