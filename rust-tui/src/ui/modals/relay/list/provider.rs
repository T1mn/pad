use super::super::super::common::truncate_modal_line;
use crate::i18n::Locale;
use crate::theme::{AgentConfig, ProviderConfig};
use crate::ui::selection::SelectionItem;

pub(super) fn provider_items(agent: Option<&AgentConfig>, locale: Locale) -> Vec<SelectionItem> {
    let Some(agent) = agent else {
        return Vec::new();
    };

    if agent.providers.is_empty() {
        return vec![SelectionItem {
            title: crate::i18n::t(locale, "relay.empty").to_string(),
            value: None,
            subtitle: None,
            keyword: None,
            disabled: true,
        }];
    }

    agent
        .providers
        .iter()
        .enumerate()
        .map(|(idx, provider)| provider_item(agent, idx, provider))
        .collect()
}

fn provider_item(agent: &AgentConfig, idx: usize, provider: &ProviderConfig) -> SelectionItem {
    let is_active = agent.active_provider == Some(idx);
    SelectionItem {
        title: provider_list_title(agent.name.as_str(), provider, is_active),
        value: None,
        subtitle: Some(provider_list_subtitle(agent.name.as_str(), provider)),
        keyword: Some(format!(
            "{} {} {}",
            provider.label,
            provider.base_url,
            provider.test_result.clone().unwrap_or_default()
        )),
        disabled: false,
    }
}

fn provider_list_title(agent_name: &str, provider: &ProviderConfig, is_active: bool) -> String {
    let marker = if is_active { "✓ " } else { "" };
    if agent_name == "opencode" {
        format!(
            "{}{} [{}]",
            marker,
            provider.label,
            provider.opencode_provider_key()
        )
    } else {
        format!("{}{}", marker, provider.label)
    }
}

fn provider_list_subtitle(agent_name: &str, provider: &ProviderConfig) -> String {
    let mut parts = Vec::new();
    if agent_name == "opencode" {
        parts.push(format!("{} models", provider.models.len()));
        parts.push(truncate_modal_line(provider.opencode_npm_package(), 24));
    }
    if !provider.base_url.trim().is_empty() {
        parts.push(truncate_modal_line(&provider.base_url, 28));
    }
    if agent_name == "codex" {
        parts.push(format!(
            "auth {}",
            super::super::layout::yes_no(provider.codex_auth_token().is_some())
        ));
    }
    if let Some(ok) = provider.test_status {
        parts.push(if ok {
            "probe ok".to_string()
        } else {
            "probe failed".to_string()
        });
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("  |  ")
    }
}
