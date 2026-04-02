use super::common::{
    backup_file, gemini_env_backup_path, gemini_env_path, gemini_settings_backup_path,
    gemini_settings_path, has_backup, parse_env_file, parse_json_object, restore_file,
    serialize_env_file, serialize_json_pretty, should_restore_standard_relay_config,
    write_text_file,
};
use crate::theme::AgentConfig;
use serde_json::json;

pub(super) fn apply_gemini_agent_config(agent: &AgentConfig) {
    let env_path = gemini_env_path();
    let settings_path = gemini_settings_path();

    if should_restore_standard_relay_config(agent) {
        restore_file(&env_path, &gemini_env_backup_path());
        restore_file(&settings_path, &gemini_settings_backup_path());
        return;
    }

    let Some(prov) = agent.active() else {
        restore_file(&env_path, &gemini_env_backup_path());
        restore_file(&settings_path, &gemini_settings_backup_path());
        return;
    };

    let env_content = std::fs::read_to_string(&env_path).unwrap_or_default();
    let settings_content = std::fs::read_to_string(&settings_path).unwrap_or_default();

    if !has_backup(&gemini_env_backup_path()) {
        let _ = backup_file(&gemini_env_backup_path(), &env_content);
    }
    if !has_backup(&gemini_settings_backup_path()) {
        let _ = backup_file(&gemini_settings_backup_path(), &settings_content);
    }

    let updated_env = update_gemini_env_config(&env_content, &prov.base_url, &prov.api_key);
    let updated_settings = update_gemini_settings_config(&settings_content);
    write_text_file(&env_path, &updated_env);
    write_text_file(&settings_path, &updated_settings);
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
