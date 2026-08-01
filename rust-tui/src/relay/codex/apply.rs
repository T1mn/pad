use super::auth::update_codex_auth_config;
use super::provider::{current_model_provider, update_codex_provider_config};
use crate::relay::common::{
    codex_auth_backup_path, codex_auth_path, codex_backup_path, codex_config_path, log_file_error,
    preserve_backup, restore_codex_auth, restore_codex_config, write_text_file,
};
use crate::theme::AgentConfig;

pub(in crate::relay) fn apply_codex_agent_config(agent: &AgentConfig) {
    let path = codex_config_path();
    let auth_path = codex_auth_path();

    if should_restore_native_codex_config(agent) {
        if let Err(error) = restore_codex_config() {
            log_file_error("restore", &path, &error);
        }
        if let Err(error) = restore_codex_auth() {
            log_file_error("restore", &auth_path, &error);
        }
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

        if let Err(error) = preserve_backup(&codex_backup_path(), &content) {
            log_file_error("backup", &codex_backup_path(), &error);
            return;
        }
        if let Err(error) = preserve_backup(&codex_auth_backup_path(), &auth_content) {
            log_file_error("backup", &codex_auth_backup_path(), &error);
            return;
        }

        let updated = update_codex_provider_config(
            &content,
            &provider_name,
            &prov.label,
            &prov.codex_base_url(),
        );
        let updated_auth = update_codex_auth_config(&auth_content, &api_key);

        if let Err(error) = write_text_file(&path, &updated) {
            log_file_error("write", &path, &error);
            return;
        }
        if let Err(error) = write_text_file(&auth_path, &updated_auth) {
            log_file_error("write", &auth_path, &error);
            return;
        }

        if current_model_provider(&content).as_deref() != Some(provider_name.as_str()) {
            crate::codex_provider_sync::enqueue_sync_to_provider(provider_name.clone());
        }
    } else {
        if let Err(error) = restore_codex_config() {
            log_file_error("restore", &path, &error);
        }
        if let Err(error) = restore_codex_auth() {
            log_file_error("restore", &auth_path, &error);
        }
    }
}

pub(in crate::relay) fn should_restore_native_codex_config(agent: &AgentConfig) -> bool {
    let Some(prov) = agent.active() else {
        return true;
    };
    prov.base_url.trim().is_empty() || prov.codex_auth_token().is_none()
}
