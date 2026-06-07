use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{AgentPanel, AgentType};
use crate::notification_inbox::NotificationDraft;
use crate::notify::NotificationRequest;
use crate::sidebar::ThreadActivityOverride;

#[derive(Clone, Debug)]
pub(super) struct PendingNotification {
    pub(super) request: NotificationRequest,
    pub(super) draft: NotificationDraft,
}

pub(super) fn completion_notification_for_panel(
    panel: &AgentPanel,
    event: &HookEvent,
    persisted_snapshot: Option<&crate::session_cache::SessionCacheSnapshot>,
) -> Option<PendingNotification> {
    if event.event != "stop" {
        return None;
    }

    let session_id = panel
        .agent_session_id
        .as_deref()
        .or_else(|| persisted_snapshot.map(|snapshot| snapshot.agent_session_id.as_str()))
        .or(event.session_id.as_deref());
    let fallback_prompt = panel
        .last_user_prompt
        .as_deref()
        .or_else(|| persisted_snapshot.and_then(|snapshot| snapshot.last_user_prompt.as_deref()))
        .or(event.prompt.as_deref());

    let request = build_completion_notification(
        &panel.agent_type,
        session_id,
        fallback_prompt,
        Some(panel.working_dir.as_str()),
    );
    Some(PendingNotification {
        draft: NotificationDraft {
            event: event.event.clone(),
            agent_type: panel.agent_type.to_string(),
            title: request.title.clone(),
            body: request.body.clone(),
            working_dir: Some(panel.working_dir.clone()),
            session_id: session_id.map(str::to_string),
            transcript_path: panel
                .transcript_path
                .clone()
                .or_else(|| {
                    persisted_snapshot.and_then(|snapshot| snapshot.transcript_path.clone())
                })
                .or_else(|| event.transcript_path.clone()),
            pane_id: Some(panel.pane_id.clone()),
        },
        request,
    })
}

pub(super) fn completion_notification_for_activity(
    activity: &ThreadActivityOverride,
    event: &HookEvent,
) -> Option<PendingNotification> {
    if event.event != "stop" {
        return None;
    }

    let session_id = activity
        .session_id
        .as_deref()
        .or(event.session_id.as_deref());
    let request = build_completion_notification(
        &activity.agent_type,
        session_id,
        activity
            .last_user_prompt
            .as_deref()
            .or(event.prompt.as_deref()),
        Some(activity.working_dir.as_str()),
    );
    Some(PendingNotification {
        draft: NotificationDraft {
            event: event.event.clone(),
            agent_type: activity.agent_type.to_string(),
            title: request.title.clone(),
            body: request.body.clone(),
            working_dir: Some(activity.working_dir.clone()),
            session_id: session_id.map(str::to_string),
            transcript_path: activity
                .transcript_path
                .clone()
                .or_else(|| event.transcript_path.clone()),
            pane_id: event.tmux.pane_id.clone(),
        },
        request,
    })
}

pub(super) fn build_completion_notification(
    agent_type: &AgentType,
    session_id: Option<&str>,
    fallback_prompt: Option<&str>,
    working_dir: Option<&str>,
) -> NotificationRequest {
    NotificationRequest {
        title: format!(
            "PAD · {} complete",
            super::notification_text::notification_agent_label(agent_type)
        ),
        body: super::notification_text::completion_notification_body(
            agent_type,
            session_id,
            fallback_prompt,
            working_dir,
        ),
    }
}

pub(super) fn emit_completion_notification(
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
