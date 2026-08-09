use super::super::*;

pub(super) fn render(config: &Config) -> String {
    let mut content = String::new();
    push_str_line(&mut content, "theme", &config.theme);
    content.push_str(&format!("auto_refresh = {}\n", config.auto_refresh));
    content.push_str(&format!("refresh_interval = {}\n", config.refresh_interval));
    push_str_line(&mut content, "language", &config.language);
    content.push_str("\n[preview]\n");
    push_str_line(&mut content, "mode", &config.preview.mode);
    content.push_str("\n[display]\n");
    push_str_line(&mut content, "session_scope", &config.display.session_scope);
    if let Some(width) = config.display.agent_panel_width {
        content.push_str(&format!("agent_panel_width = {}\n", width));
    }
    content.push_str("\n[sound]\n");
    content.push_str(&format!("enabled = {}\n", config.sound.enabled));
    push_sound_event_config(&mut content, "completion", &config.sound.completion);
    push_sound_event_config(&mut content, "approval", &config.sound.approval);
    push_sound_event_config(&mut content, "timeout", &config.sound.timeout);
    push_sound_event_config(&mut content, "failure", &config.sound.failure);
    content.push_str("\n[telegram]\n");
    content.push_str(&format!("enabled = {}\n", config.telegram.enabled));
    push_str_line(&mut content, "bot_token", &config.telegram.bot_token);
    push_str_line(&mut content, "chat_id", &config.telegram.chat_id);
    push_str_line(&mut content, "bot_username", &config.telegram.bot_username);
    push_codex(&mut content, &config.codex);
    content.push_str("\n[agent_permissions]\n");
    content.push_str(&format!(
        "codex_auto_full_access = {}\n",
        config.agent_permissions.codex_auto_full_access
    ));
    content.push_str(&format!(
        "claude_auto_full_access = {}\n",
        config.agent_permissions.claude_auto_full_access
    ));
    content.push('\n');
    push_agents(&mut content, &config.agents);
    content
}

fn push_codex(content: &mut String, codex: &CodexConfig) {
    content.push_str("\n[codex]\n");
    content.push_str(&format!("fast_mode = {}\n", codex.fast_mode));
    content.push_str(&format!("goals = {}\n", codex.goals));
    content.push_str(&format!("multi_agent = {}\n", codex.multi_agent));
    push_str_line(content, "web_search", &codex.web_search);
    for (key, value) in [
        (
            "status_line_model_with_reasoning",
            codex.status_line_model_with_reasoning,
        ),
        ("status_line_fast_mode", codex.status_line_fast_mode),
        (
            "status_line_five_hour_limit",
            codex.status_line_five_hour_limit,
        ),
        ("status_line_weekly_limit", codex.status_line_weekly_limit),
        (
            "status_line_context_remaining",
            codex.status_line_context_remaining,
        ),
        ("status_line_current_dir", codex.status_line_current_dir),
        ("jailbreak_prompt_file", codex.jailbreak_prompt_file),
        ("index_prompt_file", codex.index_prompt_file),
        ("title_summary", codex.title_summary),
        ("show_qa_preview", codex.show_qa_preview),
    ] {
        content.push_str(&format!("{key} = {value}\n"));
    }
}

fn push_agents(content: &mut String, agents: &[AgentConfig]) {
    for agent in agents {
        content.push_str("[[agents]]\n");
        push_str_line(content, "name", &agent.name);
        push_str_line(content, "cmd", &agent.cmd);
        if let Some(idx) = agent.active_provider {
            content.push_str(&format!("active_provider = {}\n", idx));
        }
        if !agent.default_model.is_empty() {
            push_str_line(content, "default_model", &agent.default_model);
        }
        if !agent.small_model.is_empty() {
            push_str_line(content, "small_model", &agent.small_model);
        }
        for provider in &agent.providers {
            push_provider(content, provider);
        }
        content.push('\n');
    }
}

fn push_provider(content: &mut String, provider: &ProviderConfig) {
    content.push_str("\n[[agents.providers]]\n");
    push_str_line(content, "label", &provider.label);
    push_str_line(content, "base_url", &provider.base_url);
    push_str_line(content, "api_key", &provider.api_key);
    if !provider.provider_key.trim().is_empty() {
        push_str_line(content, "provider_key", &provider.provider_key);
    }
    if !provider.npm_package.trim().is_empty() {
        push_str_line(content, "npm_package", &provider.npm_package);
    }
    if provider.disable_thinking {
        content.push_str("disable_thinking = true\n");
    }
    for model in &provider.models {
        content.push_str("\n[[agents.providers.models]]\n");
        push_str_line(content, "id", &model.id);
        push_str_line(content, "name", &model.name);
    }
}

fn push_sound_event_config(content: &mut String, name: &str, config: &SoundEventConfig) {
    content.push_str(&format!("\n[sound.{name}]\n"));
    content.push_str(&format!("enabled = {}\n", config.enabled));
    push_str_line(content, "preset", &config.preset);
}

/// 字符串值一律交给 toml crate 编码。手写 `value.replace('"', "\\\"")` 漏掉了反斜杠、
/// 换行和控制字符，`C:\path` 这种 api_key 会写出解析不了的文件。
fn push_str_line(content: &mut String, key: &str, value: &str) {
    content.push_str(key);
    content.push_str(" = ");
    content.push_str(&toml_string_literal(value));
    content.push('\n');
}

fn toml_string_literal(value: &str) -> String {
    toml::Value::String(value.to_string()).to_string()
}

#[cfg(test)]
#[path = "render_tests.rs"]
pub(crate) mod tests;
