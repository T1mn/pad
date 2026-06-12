mod claude;
mod codex;
mod common;
mod deepseek;
mod gemini;
mod opencode;
mod permissions;

use crate::theme::CodexConfig;
use crate::theme::{AgentConfig, AgentPermissionsConfig};
use std::path::PathBuf;

pub(crate) fn claude_base_url(raw: &str) -> String {
    claude::claude_base_url(raw)
}

/// Apply the active provider's relay/proxy config to each agent's native config files.
pub fn apply_relay_configs(agents: &[AgentConfig]) {
    for agent in agents {
        match agent.name.as_str() {
            "claude" => claude::apply_claude_agent_config(agent),
            "codex" => codex::apply_codex_agent_config(agent),
            "deepseek" | "deepseek(cc)" => deepseek::apply_deepseek_agent_config(agent),
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

pub fn write_codex_relay_export(agent: &AgentConfig) -> std::io::Result<PathBuf> {
    let path = crate::paths::relay_export_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, codex::export_codex_relay_yaml(agent))?;
    Ok(path)
}

pub fn read_codex_relay_import(
) -> Result<(Vec<crate::theme::ProviderConfig>, Option<usize>, PathBuf), String> {
    let path = crate::paths::relay_export_path();
    let (providers, active_provider) = codex::import_codex_relay_yaml(&path)?;
    Ok((providers, active_provider, path))
}

#[cfg(test)]
mod tests;
