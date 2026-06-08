use super::auth::update_codex_auth_config;
use super::provider::{current_model_provider, update_codex_provider_config};
use crate::relay::common::{
    backup_codex_auth, backup_codex_config, codex_auth_path, codex_config_path,
    has_pad_codex_auth_backup, has_pad_codex_backup, restore_codex_auth, restore_codex_config,
};
use crate::theme::AgentConfig;

pub(in crate::relay) fn apply_codex_agent_config(agent: &AgentConfig) {
    let path = codex_config_path();
    let auth_path = codex_auth_path();

    if should_restore_native_codex_config(agent) {
        restore_codex_config();
        restore_codex_auth();
        return;
    }

    if let Some(prov) = agent.active() {
        let api_key = prov.codex_auth_token().unwrap_or_default();
        let provider_name = prov.codex_provider_name();

        let content = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };
        let auth_content = if auth_path.exists() {
            std::fs::read_to_string(&auth_path).unwrap_or_default()
        } else {
            String::new()
        };

        if !has_pad_codex_backup() {
            let _ = backup_codex_config(&content);
        }
        if !has_pad_codex_auth_backup() {
            let _ = backup_codex_auth(&auth_content);
        }

        let updated = update_codex_provider_config(
            &content,
            &provider_name,
            &prov.label,
            &prov.codex_base_url(),
        );
        let updated_auth = update_codex_auth_config(&auth_content, &api_key);

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Some(parent) = auth_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, updated);
        let _ = std::fs::write(&auth_path, updated_auth);

        if current_model_provider(&content).as_deref() != Some(provider_name.as_str()) {
            crate::codex_provider_sync::enqueue_sync_to_provider(provider_name.clone());
        }
    } else {
        restore_codex_config();
        restore_codex_auth();
    }
}

pub(in crate::relay) fn should_restore_native_codex_config(agent: &AgentConfig) -> bool {
    let Some(prov) = agent.active() else {
        return true;
    };
    prov.base_url.trim().is_empty() || prov.codex_auth_token().is_none()
}
