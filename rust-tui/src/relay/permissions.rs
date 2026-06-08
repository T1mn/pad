mod claude;
mod codex;
mod json_helpers;
mod toml_helpers;

use crate::theme::{AgentConfig, AgentPermissionsConfig, CodexConfig};
use codex::CodexRuntimeOverlay;

pub(super) fn apply_runtime_overlays(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
    codex_config: &CodexConfig,
) {
    let has_codex = agents.iter().any(|agent| agent.name == "codex");
    let has_claude = agents.iter().any(|agent| agent.name == "claude");

    if has_codex {
        let status_line_items = codex_config.status_line_items();
        codex::apply_codex_runtime_overlay(CodexRuntimeOverlay {
            yolo_enabled: permissions.codex_auto_full_access,
            fast_enabled: codex_config.fast_mode,
            goals_enabled: codex_config.goals,
            multi_agent_enabled: codex_config.multi_agent,
            web_search_mode: &codex_config.web_search,
            status_line_items: &status_line_items,
            jailbreak_prompt_file_enabled: codex_config.jailbreak_prompt_file,
            index_prompt_file_enabled: codex_config.index_prompt_file,
        });
    } else {
        codex::remove_codex_runtime_overlay(CodexRuntimeOverlay {
            yolo_enabled: false,
            fast_enabled: false,
            goals_enabled: false,
            multi_agent_enabled: false,
            web_search_mode: "default",
            status_line_items: &[],
            jailbreak_prompt_file_enabled: false,
            index_prompt_file_enabled: false,
        });
    }

    if has_claude && permissions.claude_auto_full_access {
        claude::apply_claude_permission_overlay();
    } else if has_claude {
        claude::remove_claude_permission_overlay();
    }
}
