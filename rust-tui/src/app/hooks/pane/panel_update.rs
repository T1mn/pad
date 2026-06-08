use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{AgentPanel, AgentState, AgentStateSource, SessionCacheState};
use crate::session_cache::SessionCacheSnapshot;

pub(super) fn apply_panel_hook_event(
    panel: &mut AgentPanel,
    event: &HookEvent,
    panel_item_focused: bool,
) -> Option<SessionCacheSnapshot> {
    apply_event_metadata(panel, event);
    apply_event_state(panel, event, panel_item_focused);
    crate::session_continuity::record_hook_event(
        Some(&panel.agent_type),
        event,
        panel.agent_session_id.as_deref(),
        panel.transcript_path.as_deref(),
    );

    let persisted_snapshot = persist_hook_event(panel, event);
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
