use super::super::super::*;
use crate::app::state::{CodexSettingsView, RelayView, SettingsDetailKind};

pub(in crate::app::actions::settings::detail) fn apply_settings_search_route(
    app: &mut App,
    kind: SettingsDetailKind,
    query: &str,
) {
    let query = query.trim();
    if query.is_empty() {
        return;
    }

    match kind {
        SettingsDetailKind::Relay => apply_relay_route(app, query),
        SettingsDetailKind::CodexSettings => apply_codex_settings_route(app, query),
        _ => {}
    }
}

fn apply_relay_route(app: &mut App, query: &str) {
    let Some(agent_idx) = matching_agent_index(app, query) else {
        return;
    };

    app.relay_selected_agent = agent_idx;
    app.relay_selected_provider = app
        .config
        .agents
        .get(agent_idx)
        .and_then(|agent| agent.active_provider)
        .unwrap_or(0);
    app.relay_view = RelayView::ProviderList;

    if let Some(provider_idx) = matching_provider_index(app, agent_idx, query) {
        app.relay_selected_provider = provider_idx;
        app.relay_view = RelayView::DetailPane;
    }
}

fn matching_agent_index(app: &App, query: &str) -> Option<usize> {
    app.config.agents.iter().position(|agent| {
        query
            .split_whitespace()
            .any(|token| token.eq_ignore_ascii_case(agent.name.as_str()))
    })
}

fn matching_provider_index(app: &App, agent_idx: usize, query: &str) -> Option<usize> {
    let agent = app.config.agents.get(agent_idx)?;
    let agent_name = agent.name.as_str();
    let extra_tokens = query
        .split_whitespace()
        .filter(|token| !is_relay_route_token(token, agent_name))
        .collect::<Vec<_>>();
    if extra_tokens.is_empty() {
        return None;
    }

    agent.providers.iter().position(|provider| {
        let blob = format!(
            "{} {} {}",
            provider.label, provider.base_url, provider.provider_key
        );
        extra_tokens
            .iter()
            .all(|token| crate::text_match::contains_ignore_case(&blob, token))
    })
}

fn is_relay_route_token(token: &str, agent_name: &str) -> bool {
    token.eq_ignore_ascii_case("relay")
        || token.eq_ignore_ascii_case("provider")
        || token.eq_ignore_ascii_case("providers")
        || token.eq_ignore_ascii_case("settings")
        || token.eq_ignore_ascii_case(agent_name)
}

fn apply_codex_settings_route(app: &mut App, query: &str) {
    let Some(view) = codex_view_from_query(query) else {
        return;
    };
    app.codex_settings_view = view;
    app.codex_settings_category_selected = view.category_index();
    app.codex_settings_selected = 0;
}

fn codex_view_from_query(query: &str) -> Option<CodexSettingsView> {
    let q = query.to_ascii_lowercase();
    if q.contains("status") || q.contains("statusline") {
        Some(CodexSettingsView::StatusLine)
    } else if q.contains("prompt") {
        Some(CodexSettingsView::Prompts)
    } else if q.contains("preview") || q.contains("summary") {
        Some(CodexSettingsView::Preview)
    } else if q.contains("cli") || q.contains("version") || q.contains("update") {
        Some(CodexSettingsView::Cli)
    } else if q.contains("runtime")
        || q.contains("permission")
        || q.contains("yolo")
        || q.contains("fast")
        || q.contains("goal")
        || q.contains("web")
        || q.contains("search")
        || q.contains("multi")
    {
        Some(CodexSettingsView::Runtime)
    } else {
        None
    }
}
