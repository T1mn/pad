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
