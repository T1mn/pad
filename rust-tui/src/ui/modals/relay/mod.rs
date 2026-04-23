mod detail;
mod layout;
mod list;
mod popup;

use super::common::render_modal_surface;
use crate::app::state::RelayView;
use crate::app::App;
use crate::ui::layout::popup_area;
use crate::ui::selection::render::recommended_list_modal_height;
use ratatui::{layout::Rect, Frame};

pub fn draw_relay_settings(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let content_w = match app.relay_view {
        RelayView::AgentList => 58,
        RelayView::ProviderList => 76,
        RelayView::DetailPane => layout::relay_detail_width(app),
    };
    let content_h = if app.relay_view == RelayView::DetailPane {
        let selected_agent = app.config.agents.get(app.relay_selected_agent);
        let provider =
            selected_agent.and_then(|agent| agent.providers.get(app.relay_selected_provider));
        let base_lines = layout::relay_detail_base_lines(app);
        let test_lines = if app.provider_test_in_progress {
            2
        } else if provider
            .map(|prov| prov.test_result.is_some())
            .unwrap_or(false)
        {
            4
        } else {
            0
        };
        base_lines + test_lines
    } else {
        let count = if app.relay_view == RelayView::AgentList {
            app.config.agents.len() as u16
        } else {
            app.config
                .agents
                .get(app.relay_selected_agent)
                .map(|agent| agent.providers.len() as u16)
                .unwrap_or(1)
        };
        recommended_list_modal_height(count, 2, 1, 1).max(12)
    };
    let area = popup_area(content_w, content_h, f.area());
    render_modal_surface(f, area, theme);
    draw_relay_in_area(f, app, area);
}

pub(super) fn draw_relay_in_area(f: &mut Frame, app: &App, area: Rect) {
    if app.relay_view == RelayView::DetailPane {
        detail::draw_relay_detail_content(f, app, area);
    } else {
        list::draw_relay_settings_content(f, app, area);
    }
}

pub fn draw_relay_detail(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let provider =
        selected_agent.and_then(|agent| agent.providers.get(app.relay_selected_provider));
    let content_w = layout::relay_detail_width(app);
    let base_lines = layout::relay_detail_base_lines(app);
    let test_lines = if app.provider_test_in_progress {
        2
    } else if provider
        .map(|prov| prov.test_result.is_some())
        .unwrap_or(false)
    {
        4
    } else {
        0
    };
    let content_h = base_lines + test_lines;
    let area = popup_area(content_w, content_h, f.area());
    render_modal_surface(f, area, theme);
    detail::draw_relay_detail_content(f, app, area);
}
