use crate::app::App;
use crate::model::AgentType;
use crate::sidebar::SidebarThread;

pub(super) fn preview_provider_value(app: &App, thread: &SidebarThread) -> String {
    let agent_name = match thread.agent_type {
        AgentType::OpenCode => "opencode",
        _ => return agent_provider_value(app, &thread.agent_type.to_string(), thread),
    };
    agent_provider_value(app, agent_name, thread)
}

fn agent_provider_value(app: &App, agent_name: &str, thread: &SidebarThread) -> String {
    let Some(agent) = app
        .config
        .agents
        .iter()
        .find(|agent| agent.name == agent_name)
    else {
        return "—".to_string();
    };
    if let Some(session_provider_name) = thread
        .session_provider_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if let Some(provider) = agent.providers.iter().find(|provider| {
            provider_identity_for_thread(thread, provider) == session_provider_name
        }) {
            return format_provider_value(thread, provider);
        }
        return session_provider_name.to_string();
    }

    let Some(provider) = agent.active() else {
        return "—".to_string();
    };
    format_provider_value(thread, provider)
}

fn provider_identity_for_thread(
    thread: &SidebarThread,
    provider: &crate::theme::ProviderConfig,
) -> String {
    match thread.agent_type {
        AgentType::Codex => provider.codex_provider_name(),
        AgentType::OpenCode => provider.opencode_provider_key().to_string(),
        _ => provider.label.trim().to_string(),
    }
}

fn format_provider_value(
    thread: &SidebarThread,
    provider: &crate::theme::ProviderConfig,
) -> String {
    let label = provider.label.trim();
    let url = match thread.agent_type {
        AgentType::Codex => provider.codex_base_url(),
        _ => provider.base_url.trim().trim_end_matches('/').to_string(),
    };

    match (label.is_empty(), url.is_empty()) {
        (true, true) => "—".to_string(),
        (false, true) => label.to_string(),
        (true, false) => url,
        (false, false) => format!("{label} · {url}"),
    }
}
