use super::unix_now_ts;
use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{AgentState, AgentType};
use crate::sidebar::ThreadActivityOverride;

pub(super) fn normalize_codex_rollout_paths_if_needed(event: &HookEvent) {
    let Some(path) = event.transcript_path.as_deref() else {
        return;
    };
    let pad_codex_home = crate::paths::pad_codex_home_dir()
        .to_string_lossy()
        .to_string();
    if !path.starts_with(&pad_codex_home) {
        return;
    }
    if let Err(err) = crate::codex_state::normalize_pad_codex_home_rollout_paths() {
        log_debug!("hook: codex rollout path normalization failed: {}", err);
    }
}

pub(super) fn app_thread_activity_from_hook(event: &HookEvent) -> Option<ThreadActivityOverride> {
    let working_dir = event.cwd.clone()?;
    let agent_type = infer_hook_agent_type(event)?;
    let updated_at = unix_now_ts();

    let (state, is_active) = match event.event.as_str() {
        "user_prompt_submit" => (AgentState::Busy, true),
        "stop" => (AgentState::Waiting, false),
        "session_start" => (AgentState::Idle, false),
        _ => (AgentState::Idle, false),
    };

    Some(ThreadActivityOverride {
        agent_type,
        session_id: event.session_id.clone(),
        transcript_path: event.transcript_path.clone(),
        working_dir,
        state,
        is_active,
        last_user_prompt: event.prompt.clone(),
        last_assistant_message: event.last_assistant_message.clone(),
        updated_at,
    })
}

fn infer_hook_agent_type(event: &HookEvent) -> Option<AgentType> {
    if let Some(path) = event.transcript_path.as_deref() {
        if path.contains("/.codex/") || path.contains("/.pad/codex-home/") {
            return Some(AgentType::Codex);
        }
        if path.contains("/.claude/") {
            return Some(AgentType::Claude);
        }
    }

    if let Some(session_id) = event.session_id.as_deref() {
        if crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some()
        {
            return Some(AgentType::Codex);
        }
        if crate::claude_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some()
        {
            return Some(AgentType::Claude);
        }
        if crate::gemini_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some()
        {
            return Some(AgentType::Gemini);
        }
    }

    Some(AgentType::Codex)
}
