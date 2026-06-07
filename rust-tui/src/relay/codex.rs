use super::common::{
    backup_codex_auth, backup_codex_config, codex_auth_path, codex_config_path,
    has_pad_codex_auth_backup, has_pad_codex_backup, parse_toml_document, restore_codex_auth,
    restore_codex_config, serialize_toml_document,
};
use crate::theme::{AgentConfig, ProviderConfig};
use serde_json::json;
use std::path::Path;

pub(super) fn apply_codex_agent_config(agent: &AgentConfig) {
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

#[derive(Debug, Default)]
struct CodexRelayExport {
    version: u32,
    codex: CodexRelayConfig,
}

#[derive(Debug, Default)]
struct CodexRelayConfig {
    active_provider: Option<usize>,
    providers: Vec<CodexRelayProvider>,
}

#[derive(Debug, Default)]
struct CodexRelayProvider {
    label: String,
    provider_name: String,
    base_url: String,
    api_key: String,
    env_key: String,
}

pub(super) fn should_restore_native_codex_config(agent: &AgentConfig) -> bool {
    let Some(prov) = agent.active() else {
        return true;
    };
    prov.base_url.trim().is_empty() || prov.codex_auth_token().is_none()
}

fn update_codex_provider_config(
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

pub(super) fn export_codex_relay_yaml(agent: &AgentConfig) -> String {
    let mut out = String::new();
    out.push_str("version: 1\n");
    out.push_str("codex:\n");
    out.push_str("  active_provider: ");
    match agent.active_provider {
        Some(index) => {
            out.push_str(&index.to_string());
            out.push('\n');
        }
        None => out.push_str("null\n"),
    }

    if agent.providers.is_empty() {
        out.push_str("  providers: []\n");
        return out;
    }

    out.push_str("  providers:\n");
    for provider in &agent.providers {
        out.push_str("    - label: ");
        out.push_str(&yaml_string(&provider.label));
        out.push('\n');

        out.push_str("      provider_name: ");
        out.push_str(&yaml_string(&provider.codex_provider_name()));
        out.push('\n');

        out.push_str("      base_url: ");
        out.push_str(&yaml_string(&provider.codex_base_url()));
        out.push('\n');

        out.push_str("      api_key: ");
        out.push_str(&yaml_string(&provider.api_key));
        out.push('\n');

        if !provider.env_key.trim().is_empty() {
            out.push_str("      env_key: ");
            out.push_str(&yaml_string(&provider.env_key));
            out.push('\n');
        }
    }

    out
}

pub(super) fn import_codex_relay_yaml(
    path: &Path,
) -> Result<(Vec<ProviderConfig>, Option<usize>), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {}", path.display(), err))?;
    let parsed = parse_codex_relay_yaml(&content)
        .map_err(|err| format!("failed to parse {}: {}", path.display(), err))?;
    if parsed.version != 1 {
        return Err(format!(
            "unsupported relay export version {} in {}",
            parsed.version,
            path.display()
        ));
    }

    let providers = parsed
        .codex
        .providers
        .into_iter()
        .map(|provider| ProviderConfig {
            label: provider.label,
            base_url: provider.base_url,
            api_key: provider.api_key,
            env_key: provider.env_key,
            wire_api: String::new(),
            provider_key: provider.provider_name,
            npm_package: "@ai-sdk/openai-compatible".to_string(),
            models: Vec::new(),
            test_status: None,
            test_http_status: None,
            test_latency_ms: None,
            test_result: None,
        })
        .collect::<Vec<_>>();

    let active_provider = parsed
        .codex
        .active_provider
        .filter(|idx| *idx < providers.len());

    Ok((providers, active_provider))
}

fn parse_codex_relay_yaml(content: &str) -> Result<CodexRelayExport, String> {
    let mut export = CodexRelayExport::default();
    let mut saw_version = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("version:") {
            export.version = value
                .trim()
                .parse::<u32>()
                .map_err(|_| "invalid version".to_string())?;
            saw_version = true;
            continue;
        }
        if line == "codex:" || line == "providers:" || line == "providers: []" {
            continue;
        }
        if let Some(value) = line.strip_prefix("active_provider:") {
            let value = value.trim();
            export.codex.active_provider = if value.eq_ignore_ascii_case("null") {
                None
            } else {
                Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| "invalid active_provider".to_string())?,
                )
            };
            continue;
        }
        if let Some(value) = line.strip_prefix("- label:") {
            export.codex.providers.push(CodexRelayProvider {
                label: parse_yaml_string(value.trim())?,
                ..Default::default()
            });
            continue;
        }

        let Some(current) = export.codex.providers.last_mut() else {
            continue;
        };
        if let Some(value) = line.strip_prefix("label:") {
            current.label = parse_yaml_string(value.trim())?;
        } else if let Some(value) = line.strip_prefix("provider_name:") {
            current.provider_name = parse_yaml_string(value.trim())?;
        } else if let Some(value) = line.strip_prefix("base_url:") {
            current.base_url = parse_yaml_string(value.trim())?;
        } else if let Some(value) = line.strip_prefix("api_key:") {
            current.api_key = parse_yaml_string(value.trim())?;
        } else if let Some(value) = line.strip_prefix("env_key:") {
            current.env_key = parse_yaml_string(value.trim())?;
        }
    }

    if !saw_version {
        return Err("missing version".to_string());
    }

    Ok(export)
}

fn yaml_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

fn parse_yaml_string(value: &str) -> Result<String, String> {
    if value.eq_ignore_ascii_case("null") {
        return Ok(String::new());
    }
    if !(value.starts_with('"') && value.ends_with('"')) {
        return Ok(value.to_string());
    }

    let inner = &value[1..value.len().saturating_sub(1)];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => return Err("invalid escape sequence".to_string()),
        }
    }
    Ok(out)
}

fn update_codex_auth_config(content: &str, api_key: &str) -> String {
    let mut obj = serde_json::from_str::<serde_json::Value>(content).unwrap_or_else(|_| json!({}));
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

fn current_model_provider(content: &str) -> Option<String> {
    let doc = parse_toml_document(content);
    doc.get("model_provider")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}
