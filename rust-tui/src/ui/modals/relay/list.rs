mod agent {
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
}
pub(crate) mod provider;

use super::layout::relay_provider_footer_text;
use crate::app::App;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::{layout::Rect, Frame};

pub(super) fn draw_relay_settings_content(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let (title, items, selected, footer_text): (String, Vec<SelectionItem>, usize, &str) = match app
        .relay_view
    {
        crate::app::state::RelayView::AgentList => (
            crate::i18n::t(locale, "relay.title").to_string(),
            agent::agent_items(&app.config.agents),
            app.relay_selected_agent,
            crate::i18n::t(locale, "relay.footer_agent"),
        ),
        crate::app::state::RelayView::ProviderList | crate::app::state::RelayView::DetailPane => (
            format!(
                "{} / {}",
                crate::i18n::t(locale, "relay.providers_label"),
                selected_agent
                    .map(|agent| agent.name.as_str())
                    .unwrap_or("?")
            ),
            provider::provider_items(selected_agent, locale),
            app.relay_selected_provider,
            relay_provider_footer_text(app, locale),
        ),
    };

    let mut state = SelectionState {
        selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(f, area, theme, &title, &items, &state, Some(footer_text));
}
