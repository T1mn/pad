use crate::model::{AgentPanel, AgentType};

pub(in crate::session_cache) fn supports_cached_session(panel: &AgentPanel) -> bool {
    matches!(
        panel.agent_type,
        AgentType::Claude | AgentType::Codex | AgentType::Gemini | AgentType::OpenCode
    )
}

#[cfg(test)]
#[path = "support_tests.rs"]
mod tests;
