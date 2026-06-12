use super::common::{
    backup_file, claude_backup_path, claude_settings_path, has_backup, parse_json_object,
    restore_file, serialize_json_pretty, should_restore_standard_relay_config, write_text_file,
};
use crate::theme::AgentConfig;
use serde_json::json;

pub(super) fn apply_claude_agent_config(agent: &AgentConfig) {
    let path = claude_settings_path();

    if should_restore_standard_relay_config(agent) {
        restore_file(&path, &claude_backup_path());
        return;
    }

    let Some(prov) = agent.active() else {
        restore_file(&path, &claude_backup_path());
        return;
    };

    let content = std::fs::read_to_string(&path).unwrap_or_default();
    if !has_backup(&claude_backup_path()) {
        let _ = backup_file(&claude_backup_path(), &content);
    }

    let updated = update_claude_settings_config(
        &content,
        &prov.base_url,
        &prov.api_key,
        &agent.default_model,
        prov.disable_thinking,
    );
    write_text_file(&path, &updated);
}

pub(super) fn update_claude_settings_config(
    content: &str,
    base_url: &str,
    api_key: &str,
    default_model: &str,
    disable_thinking: bool,
) -> String {
    let mut obj = parse_json_object(content);
    obj.as_object_mut()
        .expect("claude settings root object")
        .remove("apiUrl");
    obj.as_object_mut()
        .expect("claude settings root object")
        .remove("apiKey");

    let env = obj
        .as_object_mut()
        .expect("claude settings root object")
        .entry("env".to_string())
        .or_insert_with(|| json!({}));
    if !env.is_object() {
        *env = json!({});
    }

    let env_obj = env.as_object_mut().expect("claude env object");
    env_obj.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        serde_json::Value::String(claude_base_url(base_url)),
    );
    env_obj.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        serde_json::Value::String(api_key.to_string()),
    );
    env_obj.remove("ANTHROPIC_API_KEY");
    env_obj.insert(
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
        serde_json::Value::String("1".to_string()),
    );
    env_obj.insert(
        "CLAUDE_CODE_ATTRIBUTION_HEADER".to_string(),
        serde_json::Value::String("0".to_string()),
    );
    env_obj.remove("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS");
    env_obj.remove("MAX_THINKING_TOKENS");
    if disable_thinking {
        env_obj.insert(
            "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS".to_string(),
            serde_json::Value::String("1".to_string()),
        );
        env_obj.insert(
            "MAX_THINKING_TOKENS".to_string(),
            serde_json::Value::String("0".to_string()),
        );
    }
    env_obj.remove("ANTHROPIC_MODEL");
    env_obj.remove("ANTHROPIC_CUSTOM_MODEL_OPTION");
    if !default_model.trim().is_empty() {
        env_obj.insert(
            "ANTHROPIC_MODEL".to_string(),
            serde_json::Value::String(default_model.trim().to_string()),
        );
        env_obj.insert(
            "ANTHROPIC_CUSTOM_MODEL_OPTION".to_string(),
            serde_json::Value::String(default_model.trim().to_string()),
        );
    }

    serialize_json_pretty(&obj)
}

pub(crate) fn claude_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    trimmed.strip_suffix("/v1").unwrap_or(trimmed).to_string()
}
