mod claude;
mod codex;
mod common;
mod gemini;
mod opencode;
mod permissions;

use crate::theme::CodexConfig;
use crate::theme::{AgentConfig, AgentPermissionsConfig};

/// Apply the active provider's relay/proxy config to each agent's native config files.
pub fn apply_relay_configs(agents: &[AgentConfig]) {
    for agent in agents {
        match agent.name.as_str() {
            "claude" => claude::apply_claude_agent_config(agent),
            "codex" => codex::apply_codex_agent_config(agent),
            "gemini-cli" | "gemini" => gemini::apply_gemini_agent_config(agent),
            "opencode" => opencode::apply_opencode_agent_config(agent),
            _ => {}
        }
    }
}

/// Apply both relay/provider config and PAD-managed runtime permission overlays.
pub fn apply_runtime_configs(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
    codex: &CodexConfig,
) {
    apply_relay_configs(agents);
    permissions::apply_runtime_overlays(agents, permissions, codex);
}

#[cfg(test)]
mod tests;
