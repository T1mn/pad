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
        value: provider_list_value(agent.name.as_str(), provider),
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
    let mut subtitle = String::new();
    if agent_name == "opencode" {
        push_subtitle_part(&mut subtitle, &format!("{} models", provider.models.len()));
        push_subtitle_part(
            &mut subtitle,
            &truncate_modal_line(provider.opencode_npm_package(), 24),
        );
    }
    if !provider.base_url.trim().is_empty() {
        push_subtitle_part(&mut subtitle, &truncate_modal_line(&provider.base_url, 28));
    }
    if agent_name == "codex" {
        push_subtitle_part(
            &mut subtitle,
            &format!(
                "auth {}",
                super::super::layout::yes_no(provider.codex_auth_token().is_some())
            ),
        );
    }
    if !matches!(agent_name, "claude" | "codex") {
        if let Some(ok) = provider.test_status {
            let label = if ok { "probe ok" } else { "probe failed" };
            if let Some(latency_ms) = provider.test_latency_ms {
                push_subtitle_part(&mut subtitle, &format!("{label} · {latency_ms} ms"));
            } else {
                push_subtitle_part(&mut subtitle, label);
            }
        }
    }
    if subtitle.is_empty() {
        "-".to_string()
    } else {
        subtitle
    }
}

fn provider_list_value(agent_name: &str, provider: &ProviderConfig) -> Option<String> {
    if !matches!(agent_name, "claude" | "codex") {
        return None;
    }

    let result = provider.test_result.as_deref()?;
    if result.contains("Testing") {
        return Some("testing".to_string());
    }

    match provider.test_status {
        Some(true) => {
            let first = provider
                .test_latency_ms
                .or_else(|| number_after(result, "first output "));
            let total = number_after(result, "complete ");
            match (first, total) {
                (Some(first), Some(total)) => Some(format!("{first}/{total} ms")),
                (Some(first), None) => Some(format!("{first} ms")),
                _ => Some("ok".to_string()),
            }
        }
        Some(false) => provider
            .test_http_status
            .map(|status| format!("失败 {status}"))
            .or_else(|| Some("失败".to_string())),
        None => Some("-".to_string()),
    }
}

fn number_after(text: &str, marker: &str) -> Option<u64> {
    let start = text.find(marker)? + marker.len();
    let digits = text[start..]
        .chars()
        .skip_while(|ch| ch.is_whitespace())
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn push_subtitle_part(subtitle: &mut String, part: &str) {
    if !subtitle.is_empty() {
        subtitle.push_str("  |  ");
    }
    subtitle.push_str(part);
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
