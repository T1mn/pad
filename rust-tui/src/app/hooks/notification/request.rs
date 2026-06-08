use crate::model::AgentType;
use crate::notify::NotificationRequest;

pub(in crate::app::hooks) fn build_completion_notification(
    agent_type: &AgentType,
    session_id: Option<&str>,
    fallback_prompt: Option<&str>,
    working_dir: Option<&str>,
) -> NotificationRequest {
    NotificationRequest {
        title: format!(
            "PAD · {} complete",
            super::super::notification_text::notification_agent_label(agent_type)
        ),
        body: super::super::notification_text::completion_notification_body(
            agent_type,
            session_id,
            fallback_prompt,
            working_dir,
        ),
    }
}
