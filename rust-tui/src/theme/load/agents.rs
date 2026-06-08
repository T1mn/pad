use crate::theme::{
    normalize_provider_key, AgentConfig, Config, OpenCodeModelConfig, ProviderConfig,
};
use std::collections::HashMap;

pub(super) fn apply_agents(table: &HashMap<String, toml::Value>, config: &mut Config) {
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
            parsed.push(super::super::config::default_agent("opencode"));
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
