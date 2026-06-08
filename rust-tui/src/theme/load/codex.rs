use crate::theme::{CodexConfig, Config};

pub(super) fn apply_codex(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(codex)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = codex.get("fast_mode") {
        config.codex.fast_mode = *enabled;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("goals") {
        config.codex.goals = *enabled;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("multi_agent") {
        config.codex.multi_agent = *enabled;
    }
    if let Some(toml::Value::String(mode)) = codex.get("web_search") {
        config.codex.web_search = CodexConfig::normalized_web_search(mode);
    }
    let legacy_status_line = codex
        .get("status_line")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let mut explicit_status_line_item = false;
    if let Some(toml::Value::Boolean(enabled)) = codex.get("status_line_model_with_reasoning") {
        config.codex.status_line_model_with_reasoning = *enabled;
        explicit_status_line_item = true;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("status_line_fast_mode") {
        config.codex.status_line_fast_mode = *enabled;
        explicit_status_line_item = true;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("status_line_five_hour_limit") {
        config.codex.status_line_five_hour_limit = *enabled;
        explicit_status_line_item = true;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("status_line_weekly_limit") {
        config.codex.status_line_weekly_limit = *enabled;
        explicit_status_line_item = true;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("status_line_context_remaining") {
        config.codex.status_line_context_remaining = *enabled;
        explicit_status_line_item = true;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("status_line_current_dir") {
        config.codex.status_line_current_dir = *enabled;
        explicit_status_line_item = true;
    }
    if legacy_status_line && !explicit_status_line_item {
        config.codex.status_line_model_with_reasoning = true;
        config.codex.status_line_context_remaining = true;
        config.codex.status_line_current_dir = true;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex
        .get("jailbreak_prompt_file")
        .or_else(|| codex.get("prompt_file"))
    {
        config.codex.jailbreak_prompt_file = *enabled;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("index_prompt_file") {
        config.codex.index_prompt_file = *enabled;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("title_summary") {
        config.codex.title_summary = *enabled;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("show_qa_preview") {
        config.codex.show_qa_preview = *enabled;
    }
}
