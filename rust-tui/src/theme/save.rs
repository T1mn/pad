use super::*;

impl Config {
    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut content = String::new();
        content.push_str(&format!("theme = \"{}\"\n", self.theme));
        content.push_str(&format!("auto_refresh = {}\n", self.auto_refresh));
        content.push_str(&format!("refresh_interval = {}\n", self.refresh_interval));
        content.push_str(&format!("language = \"{}\"\n", self.language));
        content.push_str("\n[desired_agent_style]\n");
        content.push_str(&format!("zoom = \"{}\"\n", self.desired_agent_style.zoom));
        content.push_str(&format!(
            "status = \"{}\"\n",
            self.desired_agent_style.status
        ));
        content.push_str("\n[preview]\n");
        content.push_str(&format!("mode = \"{}\"\n", self.preview.mode));
        content.push_str("\n[display]\n");
        content.push_str(&format!(
            "session_scope = \"{}\"\n",
            self.display.session_scope
        ));
        content.push_str("\n[sound]\n");
        content.push_str(&format!("enabled = {}\n", self.sound.enabled));
        push_sound_event_config(&mut content, "completion", &self.sound.completion);
        push_sound_event_config(&mut content, "approval", &self.sound.approval);
        push_sound_event_config(&mut content, "timeout", &self.sound.timeout);
        push_sound_event_config(&mut content, "failure", &self.sound.failure);
        content.push_str("\n[telegram]\n");
        content.push_str(&format!("enabled = {}\n", self.telegram.enabled));
        push_escaped_line(&mut content, "bot_token", &self.telegram.bot_token);
        push_escaped_line(&mut content, "chat_id", &self.telegram.chat_id);
        push_escaped_line(&mut content, "bot_username", &self.telegram.bot_username);
        content.push_str("\n[codex]\n");
        content.push_str(&format!("fast_mode = {}\n", self.codex.fast_mode));
        content.push_str(&format!("goals = {}\n", self.codex.goals));
        content.push_str(&format!("multi_agent = {}\n", self.codex.multi_agent));
        content.push_str(&format!("web_search = \"{}\"\n", self.codex.web_search));
        content.push_str(&format!(
            "status_line_model_with_reasoning = {}\n",
            self.codex.status_line_model_with_reasoning
        ));
        content.push_str(&format!(
            "status_line_fast_mode = {}\n",
            self.codex.status_line_fast_mode
        ));
        content.push_str(&format!(
            "status_line_context_remaining = {}\n",
            self.codex.status_line_context_remaining
        ));
        content.push_str(&format!(
            "status_line_current_dir = {}\n",
            self.codex.status_line_current_dir
        ));
        content.push_str(&format!(
            "jailbreak_prompt_file = {}\n",
            self.codex.jailbreak_prompt_file
        ));
        content.push_str(&format!(
            "index_prompt_file = {}\n",
            self.codex.index_prompt_file
        ));
        content.push_str(&format!("title_summary = {}\n", self.codex.title_summary));
        content.push_str(&format!(
            "show_qa_preview = {}\n",
            self.codex.show_qa_preview
        ));
        content.push_str("\n[agent_permissions]\n");
        content.push_str(&format!(
            "codex_auto_full_access = {}\n",
            self.agent_permissions.codex_auto_full_access
        ));
        content.push_str(&format!(
            "claude_auto_full_access = {}\n",
            self.agent_permissions.claude_auto_full_access
        ));
        content.push('\n');

        for agent in &self.agents {
            content.push_str("[[agents]]\n");
            push_escaped_line(&mut content, "name", &agent.name);
            push_escaped_line(&mut content, "cmd", &agent.cmd);
            if let Some(idx) = agent.active_provider {
                content.push_str(&format!("active_provider = {}\n", idx));
            }
            if !agent.default_model.is_empty() {
                push_escaped_line(&mut content, "default_model", &agent.default_model);
            }
            if !agent.small_model.is_empty() {
                push_escaped_line(&mut content, "small_model", &agent.small_model);
            }
            for provider in &agent.providers {
                push_provider(&mut content, provider);
            }
            content.push('\n');
        }

        let _ = std::fs::write(&path, content);
    }
}

fn push_provider(content: &mut String, provider: &ProviderConfig) {
    content.push_str("\n[[agents.providers]]\n");
    push_escaped_line(content, "label", &provider.label);
    push_escaped_line(content, "base_url", &provider.base_url);
    push_escaped_line(content, "api_key", &provider.api_key);
    if !provider.provider_key.trim().is_empty() {
        push_escaped_line(content, "provider_key", &provider.provider_key);
    }
    if !provider.npm_package.trim().is_empty() {
        push_escaped_line(content, "npm_package", &provider.npm_package);
    }
    for model in &provider.models {
        content.push_str("\n[[agents.providers.models]]\n");
        push_escaped_line(content, "id", &model.id);
        push_escaped_line(content, "name", &model.name);
    }
}

fn push_sound_event_config(content: &mut String, name: &str, config: &SoundEventConfig) {
    content.push_str(&format!("\n[sound.{name}]\n"));
    content.push_str(&format!("enabled = {}\n", config.enabled));
    push_escaped_line(content, "preset", &config.preset);
}

fn push_escaped_line(content: &mut String, key: &str, value: &str) {
    content.push_str(&format!("{key} = \"{}\"\n", value.replace('"', "\\\"")));
}
