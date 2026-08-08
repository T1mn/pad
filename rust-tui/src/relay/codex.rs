mod apply {
    use super::auth::update_codex_auth_config;
    use super::provider::{current_model_provider, update_codex_provider_config};
    use crate::relay::common::{
        codex_auth_backup_path, codex_auth_path, codex_backup_path, codex_config_path,
        log_file_error, preserve_backup, restore_codex_auth, restore_codex_config, write_text_file,
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
}
mod auth {
    use serde_json::json;

    pub(super) fn update_codex_auth_config(content: &str, api_key: &str) -> String {
        let mut obj =
            serde_json::from_str::<serde_json::Value>(content).unwrap_or_else(|_| json!({}));
        if !obj.is_object() {
            obj = json!({});
        }
        obj["auth_mode"] = serde_json::Value::String("apikey".to_string());
        obj["OPENAI_API_KEY"] = serde_json::Value::String(api_key.to_string());
        let mut serialized = serde_json::to_string_pretty(&obj).unwrap_or_default();
        if !serialized.ends_with('\n') {
            serialized.push('\n');
        }
        serialized
    }
}
mod provider {
    use crate::relay::common::{parse_toml_document, serialize_toml_document};

    pub(super) fn update_codex_provider_config(
        content: &str,
        provider_name: &str,
        provider_label: &str,
        base_url: &str,
    ) -> String {
        let mut doc = parse_toml_document(content);

        let root = doc.as_table_mut().expect("root toml value must be a table");
        upsert_codex_provider_config(root, provider_name, provider_label, base_url);

        serialize_toml_document(&doc)
    }

    fn upsert_codex_provider_config(
        root: &mut toml::map::Map<String, toml::Value>,
        provider_name: &str,
        provider_label: &str,
        base_url: &str,
    ) {
        root.insert(
            "model_provider".to_string(),
            toml::Value::String(provider_name.to_string()),
        );

        let providers = root
            .entry("model_providers")
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

        if !providers.is_table() {
            *providers = toml::Value::Table(toml::map::Map::new());
        }

        let providers_table = providers
            .as_table_mut()
            .expect("model_providers must be a table");
        let provider_entry = providers_table
            .entry(provider_name.to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

        if !provider_entry.is_table() {
            *provider_entry = toml::Value::Table(toml::map::Map::new());
        }

        let provider_table = provider_entry
            .as_table_mut()
            .expect("provider entry must be a table");
        provider_table.insert(
            "base_url".to_string(),
            toml::Value::String(base_url.to_string()),
        );
        provider_table.insert(
            "name".to_string(),
            toml::Value::String(provider_label.to_string()),
        );
        provider_table.insert(
            "requires_openai_auth".to_string(),
            toml::Value::Boolean(true),
        );
        provider_table.remove("env_key");
    }

    pub(super) fn current_model_provider(content: &str) -> Option<String> {
        let doc = parse_toml_document(content);
        doc.get("model_provider")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
    }
}
mod yaml;

pub(super) use apply::apply_codex_agent_config;
#[cfg(test)]
pub(super) use apply::should_restore_native_codex_config;
pub(super) use yaml::{export_codex_relay_yaml, import_codex_relay_yaml};
