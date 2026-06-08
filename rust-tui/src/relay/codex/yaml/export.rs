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
