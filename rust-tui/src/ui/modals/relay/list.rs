mod agent;
mod provider;

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
