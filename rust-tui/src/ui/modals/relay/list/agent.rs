use super::super::super::common::truncate_modal_line;
use crate::theme::AgentConfig;
use crate::ui::selection::SelectionItem;

pub(super) fn agent_items(agents: &[AgentConfig]) -> Vec<SelectionItem> {
    agents.iter().map(agent_item).collect()
}

fn agent_item(agent: &AgentConfig) -> SelectionItem {
    let active_label = agent.active().map(active_label).unwrap_or_else(|| {
        if agent.name == "opencode" && !agent.default_model.is_empty() {
            agent.default_model.clone()
        } else {
            "none".to_string()
        }
    });

    SelectionItem {
        title: agent.name.clone(),
        value: None,
        subtitle: Some(agent_subtitle(agent, &active_label)),
        keyword: Some(format!("{} {}", agent.name, active_label)),
        detail: None,
        disabled: false,
    }
}

fn active_label(provider: &crate::theme::ProviderConfig) -> String {
    provider.label.clone()
}

fn agent_subtitle(agent: &AgentConfig, active_label: &str) -> String {
    if agent.name == "opencode" {
        let model = if agent.default_model.is_empty() {
            "none".to_string()
        } else {
            truncate_modal_line(&agent.default_model, 24)
        };
        let small = if agent.small_model.is_empty() {
            "none".to_string()
        } else {
            truncate_modal_line(&agent.small_model, 20)
        };
        format!(
            "model: {}  ·  small: {}  ·  {} providers",
            model,
            small,
            agent.providers.len()
        )
    } else {
        format!(
            "active: {}  ·  {} providers",
            active_label,
            agent.providers.len()
        )
    }
}
