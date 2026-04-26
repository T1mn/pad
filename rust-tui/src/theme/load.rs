use super::*;

impl Config {
    pub fn load() -> Self {
        let Some(load_path) = Self::resolved_config_path() else {
            return Self::default();
        };
        Self::load_from_path(&load_path).unwrap_or_default()
    }

    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| format!("read {} failed: {err}", path.display()))?;
        let table: HashMap<String, toml::Value> = toml::from_str(&content)
            .map_err(|err| format!("parse {} failed: {err}", path.display()))?;

        let mut config = Self::default();
        apply_root_fields(&table, &mut config);
        apply_section_fields(&table, &mut config);
        apply_agents(&table, &mut config);
        Ok(config)
    }
}

fn apply_root_fields(table: &HashMap<String, toml::Value>, config: &mut Config) {
    if let Some(toml::Value::String(theme)) = table.get("theme") {
        config.theme = theme.clone();
    }
    if let Some(toml::Value::Boolean(auto)) = table.get("auto_refresh") {
        config.auto_refresh = *auto;
    }
    if let Some(toml::Value::Integer(interval)) = table.get("refresh_interval") {
        config.refresh_interval = *interval as u64;
    }
    if let Some(toml::Value::String(lang)) = table.get("language") {
        config.language = lang.clone();
    }
    if let Some(toml::Value::String(sb)) = table.get("status_bar") {
        config.desired_agent_style.status = match sb.as_str() {
            "hidden" => "hide".to_string(),
            "show" => "show".to_string(),
            other => other.to_string(),
        };
    }
}

fn apply_section_fields(table: &HashMap<String, toml::Value>, config: &mut Config) {
    apply_desired_style(table.get("desired_agent_style"), config);
    apply_preview(table.get("preview"), config);
    apply_display(table.get("display"), config);
    apply_sound(table.get("sound"), config);
    apply_telegram(table.get("telegram"), config);
    apply_codex(table.get("codex"), config);
    apply_permissions(table.get("agent_permissions"), config);
}

fn apply_desired_style(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(das)) = value else {
        return;
    };
    if let Some(toml::Value::String(z)) = das.get("zoom") {
        config.desired_agent_style.zoom = z.clone();
    }
    if let Some(toml::Value::String(s)) = das.get("status") {
        config.desired_agent_style.status = s.clone();
    }
}

fn apply_preview(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(preview)) = value else {
        return;
    };
    if let Some(toml::Value::String(mode)) = preview.get("mode") {
        config.preview.mode = match mode.as_str() {
            "tmux" => "tmux".to_string(),
            "session" => "session".to_string(),
            _ => "auto".to_string(),
        };
    }
}

fn apply_display(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(display)) = value else {
        return;
    };
    if let Some(toml::Value::String(scope)) = display.get("session_scope") {
        config.display.session_scope = match scope.as_str() {
            "all" => "all".to_string(),
            _ => "live".to_string(),
        };
    }
}

fn apply_sound(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(sound)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = sound.get("enabled") {
        config.sound.enabled = *enabled;
    }
    load_sound_event_config(
        sound.get("completion"),
        &mut config.sound.completion,
        crate::sound::SoundEvent::Completion,
    );
    load_sound_event_config(
        sound.get("approval"),
        &mut config.sound.approval,
        crate::sound::SoundEvent::Approval,
    );
    load_sound_event_config(
        sound.get("timeout"),
        &mut config.sound.timeout,
        crate::sound::SoundEvent::Timeout,
    );
    load_sound_event_config(
        sound.get("failure"),
        &mut config.sound.failure,
        crate::sound::SoundEvent::Failure,
    );
}

fn apply_telegram(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(telegram)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = telegram.get("enabled") {
        config.telegram.enabled = *enabled;
    }
    if let Some(toml::Value::String(token)) = telegram.get("bot_token") {
        config.telegram.bot_token = token.clone();
    }
    if let Some(toml::Value::String(chat_id)) = telegram.get("chat_id") {
        config.telegram.chat_id = chat_id.clone();
    }
    if let Some(toml::Value::String(bot_username)) = telegram.get("bot_username") {
        config.telegram.bot_username = bot_username.clone();
    }
}

fn apply_codex(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(codex)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = codex.get("fast_mode") {
        config.codex.fast_mode = *enabled;
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
    if let Some(toml::Value::Boolean(enabled)) = codex.get("prompt_file") {
        config.codex.prompt_file = *enabled;
    }
    if let Some(toml::Value::Boolean(enabled)) = codex.get("title_summary") {
        config.codex.title_summary = *enabled;
    }
}

fn apply_permissions(value: Option<&toml::Value>, config: &mut Config) {
    let Some(toml::Value::Table(agent_permissions)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = agent_permissions.get("codex_auto_full_access") {
        config.agent_permissions.codex_auto_full_access = *enabled;
    }
    if let Some(toml::Value::Boolean(enabled)) = agent_permissions.get("claude_auto_full_access") {
        config.agent_permissions.claude_auto_full_access = *enabled;
    }
}

fn apply_agents(table: &HashMap<String, toml::Value>, config: &mut Config) {
    let Some(toml::Value::Array(agents)) = table.get("agents") else {
        return;
    };

    let mut parsed = Vec::new();
    for agent in agents {
        if let Some(parsed_agent) = parse_agent(agent) {
            parsed.push(parsed_agent);
        }
    }

    if !parsed.is_empty() {
        if !parsed.iter().any(|agent| agent.name == "opencode") {
            parsed.push(super::config::default_agent("opencode"));
        }
        config.agents = parsed;
    }
}

fn parse_agent(agent: &toml::Value) -> Option<AgentConfig> {
    let toml::Value::Table(t) = agent else {
        return None;
    };
    let (Some(toml::Value::String(name)), Some(toml::Value::String(cmd))) =
        (t.get("name"), t.get("cmd"))
    else {
        return None;
    };

    let legacy_url = string_field(t, "base_url");
    let legacy_key = string_field(t, "api_key");
    let mut parsed_agent = AgentConfig {
        name: name.clone(),
        cmd: cmd.clone(),
        providers: parse_providers(t),
        active_provider: parse_active_provider(t),
        default_model: string_field(t, "default_model").unwrap_or_default(),
        small_model: string_field(t, "small_model").unwrap_or_default(),
        base_url: legacy_url.clone(),
        api_key: legacy_key.clone(),
    };

    if parsed_agent.providers.is_empty() && (legacy_url.is_some() || legacy_key.is_some()) {
        parsed_agent.providers.push(ProviderConfig {
            label: "default".to_string(),
            base_url: legacy_url.unwrap_or_default(),
            api_key: legacy_key.unwrap_or_default(),
            env_key: String::new(),
            wire_api: "responses".to_string(),
            provider_key: "default".to_string(),
            npm_package: "@ai-sdk/openai-compatible".to_string(),
            models: Vec::new(),
            test_status: None,
            test_http_status: None,
            test_latency_ms: None,
            test_result: None,
        });
        parsed_agent.active_provider = Some(0);
    }

    if parsed_agent
        .active_provider
        .is_some_and(|idx| idx >= parsed_agent.providers.len())
    {
        parsed_agent.active_provider = None;
    }

    if parsed_agent.name == "opencode" {
        parsed_agent.repair_opencode_model_refs();
    }
    Some(parsed_agent)
}

fn parse_providers(table: &toml::map::Map<String, toml::Value>) -> Vec<ProviderConfig> {
    table
        .get("providers")
        .and_then(|value| value.as_array())
        .map(|items| items.iter().filter_map(parse_provider).collect())
        .unwrap_or_default()
}

fn parse_provider(value: &toml::Value) -> Option<ProviderConfig> {
    let table = value.as_table()?;
    let label = string_field(table, "label").unwrap_or_default();
    Some(ProviderConfig {
        base_url: string_field(table, "base_url").unwrap_or_default(),
        api_key: string_field(table, "api_key").unwrap_or_default(),
        env_key: string_field(table, "env_key").unwrap_or_default(),
        wire_api: string_field(table, "wire_api").unwrap_or_else(|| "responses".to_string()),
        provider_key: string_field(table, "provider_key")
            .unwrap_or_else(|| normalize_provider_key(&label)),
        npm_package: string_field(table, "npm_package")
            .unwrap_or_else(|| "@ai-sdk/openai-compatible".to_string()),
        models: parse_models(table),
        label,
        test_status: None,
        test_http_status: None,
        test_latency_ms: None,
        test_result: None,
    })
}

fn parse_models(table: &toml::map::Map<String, toml::Value>) -> Vec<OpenCodeModelConfig> {
    table
        .get("models")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let table = item.as_table()?;
                    let id = string_field(table, "id").unwrap_or_default();
                    let name = string_field(table, "name").unwrap_or_default();
                    (!id.trim().is_empty()).then_some(OpenCodeModelConfig { id, name })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_active_provider(table: &toml::map::Map<String, toml::Value>) -> Option<usize> {
    table
        .get("active_provider")
        .and_then(|value| value.as_integer())
        .map(|idx| idx as usize)
}

fn string_field(table: &toml::map::Map<String, toml::Value>, key: &str) -> Option<String> {
    table.get(key).and_then(|value| match value {
        toml::Value::String(value) => Some(value.clone()),
        _ => None,
    })
}

fn load_sound_event_config(
    value: Option<&toml::Value>,
    target: &mut SoundEventConfig,
    event: crate::sound::SoundEvent,
) {
    let Some(toml::Value::Table(table)) = value else {
        return;
    };
    if let Some(toml::Value::Boolean(enabled)) = table.get("enabled") {
        target.enabled = *enabled;
    }
    if let Some(toml::Value::String(preset)) = table.get("preset") {
        target.preset = SoundEventConfig::normalize_preset_for(event, preset);
    }
}
