mod export {
    use super::string::yaml_string;
    use crate::theme::AgentConfig;

    pub(in crate::relay) fn export_codex_relay_yaml(agent: &AgentConfig) -> String {
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
}
mod import {
    use super::parse::parse_codex_relay_yaml;
    use crate::theme::ProviderConfig;
    use std::path::Path;

    pub(in crate::relay) fn import_codex_relay_yaml(
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
                disable_thinking: false,
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
}
mod model {
    #[derive(Debug, Default)]
    pub(super) struct CodexRelayExport {
        pub(super) version: u32,
        pub(super) codex: CodexRelayConfig,
    }

    #[derive(Debug, Default)]
    pub(super) struct CodexRelayConfig {
        pub(super) active_provider: Option<usize>,
        pub(super) providers: Vec<CodexRelayProvider>,
    }

    #[derive(Debug, Default)]
    pub(super) struct CodexRelayProvider {
        pub(super) label: String,
        pub(super) provider_name: String,
        pub(super) base_url: String,
        pub(super) api_key: String,
        pub(super) env_key: String,
    }
}
mod parse {
    use super::model::{CodexRelayExport, CodexRelayProvider};
    use super::string::parse_yaml_string;

    pub(super) fn parse_codex_relay_yaml(content: &str) -> Result<CodexRelayExport, String> {
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
}
mod string {
    pub(super) fn yaml_string(value: &str) -> String {
        let escaped = value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        format!("\"{escaped}\"")
    }

    pub(super) fn parse_yaml_string(value: &str) -> Result<String, String> {
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
}

pub(in crate::relay) use export::export_codex_relay_yaml;
pub(in crate::relay) use import::import_codex_relay_yaml;
