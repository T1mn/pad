mod draft;
mod emit {
    use crate::log_debug;
    use crate::notify::NotificationRequest;

    pub(in crate::app::hooks) fn emit_completion_notification(
        config: &crate::theme::Config,
        request: NotificationRequest,
    ) {
        match crate::notify::notify_completion(&request) {
            Ok(true) => {}
            Ok(false) => {
                log_debug!("notification: skipped (no supported desktop backend)");
            }
            Err(err) => {
                log_debug!("notification: failed to dispatch: {}", err);
            }
        }
        match crate::sound::play_event(&config.sound, crate::sound::SoundEvent::Completion) {
            Ok(true) => {}
            Ok(false) => {}
            Err(err) => {
                log_debug!("sound: failed to play completion sound: {}", err);
            }
        }
    }
}
mod request {
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
}

use crate::notification_inbox::NotificationDraft;
use crate::notify::NotificationRequest;

pub(super) use draft::{completion_notification_for_activity, completion_notification_for_panel};
pub(super) use emit::emit_completion_notification;
#[cfg(test)]
pub(super) use request::build_completion_notification;

#[derive(Clone, Debug)]
pub(super) struct PendingNotification {
    pub(super) request: NotificationRequest,
    pub(super) draft: NotificationDraft,
}
