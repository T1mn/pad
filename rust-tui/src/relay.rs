mod claude;
mod codex;
mod common;
mod deepseek;
mod gemini {
    use super::common::{
        gemini_env_backup_path, gemini_env_path, gemini_settings_backup_path, gemini_settings_path,
        log_file_error, parse_env_file, parse_json_object, preserve_backup, restore_file,
        serialize_env_file, serialize_json_pretty, should_restore_standard_relay_config,
        write_text_file,
    };
    use crate::theme::AgentConfig;
    use serde_json::json;

    pub(super) fn apply_gemini_agent_config(agent: &AgentConfig) {
        let env_path = gemini_env_path();
        let settings_path = gemini_settings_path();

        if should_restore_standard_relay_config(agent) {
            if let Err(error) = restore_file(&env_path, &gemini_env_backup_path()) {
                log_file_error("restore", &env_path, &error);
            }
            if let Err(error) = restore_file(&settings_path, &gemini_settings_backup_path()) {
                log_file_error("restore", &settings_path, &error);
            }
            return;
        }

        let Some(prov) = agent.active() else {
            if let Err(error) = restore_file(&env_path, &gemini_env_backup_path()) {
                log_file_error("restore", &env_path, &error);
            }
            if let Err(error) = restore_file(&settings_path, &gemini_settings_backup_path()) {
                log_file_error("restore", &settings_path, &error);
            }
            return;
        };

        let env_content = std::fs::read_to_string(&env_path).unwrap_or_default();
        let settings_content = std::fs::read_to_string(&settings_path).unwrap_or_default();

        if let Err(error) = preserve_backup(&gemini_env_backup_path(), &env_content) {
            log_file_error("backup", &gemini_env_backup_path(), &error);
            return;
        }
        if let Err(error) = preserve_backup(&gemini_settings_backup_path(), &settings_content) {
            log_file_error("backup", &gemini_settings_backup_path(), &error);
            return;
        }

        let updated_env = update_gemini_env_config(&env_content, &prov.base_url, &prov.api_key);
        let updated_settings = update_gemini_settings_config(&settings_content);
        if let Err(error) = write_text_file(&env_path, &updated_env) {
            log_file_error("write", &env_path, &error);
            return;
        }
        if let Err(error) = write_text_file(&settings_path, &updated_settings) {
            log_file_error("write", &settings_path, &error);
        }
    }

    pub(super) fn update_gemini_settings_config(content: &str) -> String {
        let mut obj = parse_json_object(content);
        obj.as_object_mut()
            .expect("gemini settings root object")
            .remove("apiUrl");
        obj.as_object_mut()
            .expect("gemini settings root object")
            .remove("apiKey");

        let security = obj
            .as_object_mut()
            .expect("gemini settings root object")
            .entry("security".to_string())
            .or_insert_with(|| json!({}));
        if !security.is_object() {
            *security = json!({});
        }

        let auth = security
            .as_object_mut()
            .expect("gemini security object")
            .entry("auth".to_string())
            .or_insert_with(|| json!({}));
        if !auth.is_object() {
            *auth = json!({});
        }

        auth.as_object_mut().expect("gemini auth object").insert(
            "selectedType".to_string(),
            serde_json::Value::String("apiKey".to_string()),
        );

        serialize_json_pretty(&obj)
    }

    pub(super) fn update_gemini_env_config(content: &str, base_url: &str, api_key: &str) -> String {
        let mut env = parse_env_file(content);
        env.insert("GOOGLE_GEMINI_BASE_URL".to_string(), base_url.to_string());
        env.insert("GEMINI_API_KEY".to_string(), api_key.to_string());
        serialize_env_file(&env)
    }
}
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

/// Apply PAD-managed runtime permission overlays without rewriting provider live configs.
///
/// Startup and other non-relay UI paths should call this so OpenCode/Claude/etc.
/// configs managed by external tools (e.g. CC-Switch) are left alone.
pub fn apply_runtime_overlays(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
    codex: &CodexConfig,
) {
    permissions::apply_runtime_overlays(agents, permissions, codex);
}

/// Apply both relay/provider config and PAD-managed runtime permission overlays.
///
/// Use only when the user (or an explicit config reload) changed PAD relay settings.
pub fn apply_runtime_configs(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
    codex: &CodexConfig,
) {
    apply_relay_configs(agents);
    apply_runtime_overlays(agents, permissions, codex);
}

pub fn write_codex_relay_export(agent: &AgentConfig) -> std::io::Result<PathBuf> {
    let path = crate::paths::relay_export_path();
    crate::atomic_file::write_private(&path, codex::export_codex_relay_yaml(agent))?;
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
