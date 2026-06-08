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
