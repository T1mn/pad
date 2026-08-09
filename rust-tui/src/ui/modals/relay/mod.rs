pub(crate) mod detail;
mod layout {
    use crate::app::App;
    use crate::i18n::Locale;
    use crate::theme::Theme;
    use ratatui::style::Color;

    pub(super) fn relay_detail_width(app: &App) -> u16 {
        match app
            .config
            .agents
            .get(app.relay_selected_agent)
            .map(|agent| agent.name.as_str())
        {
            Some("codex") => 82,
            Some("opencode") => 78,
            _ => 68,
        }
    }

    pub(super) fn relay_detail_base_lines(app: &App) -> u16 {
        match app
            .config
            .agents
            .get(app.relay_selected_agent)
            .map(|agent| agent.name.as_str())
        {
            Some("codex") => 14,
            Some("claude") => 17,
            Some("opencode") => 22,
            _ => 14,
        }
    }

    pub(super) fn relay_provider_footer_text<'a>(app: &App, locale: Locale) -> &'a str {
        match app
            .config
            .agents
            .get(app.relay_selected_agent)
            .map(|agent| agent.name.as_str())
        {
            Some("claude" | "codex") => crate::i18n::t(locale, "relay.footer_provider_codex"),
            Some("opencode") => crate::i18n::t(locale, "relay.footer_provider_opencode"),
            _ => crate::i18n::t(locale, "relay.footer_provider"),
        }
    }

    pub(super) fn relay_detail_footer_text<'a>(app: &App, locale: Locale) -> &'a str {
        if app.relay_popup_mode != crate::app::state::RelayPopupMode::None {
            match app.relay_popup_mode {
                crate::app::state::RelayPopupMode::OpenCodeModels => {
                    if app.relay_popup_editing {
                        crate::i18n::t(locale, "relay.footer_edit")
                    } else {
                        crate::i18n::t(locale, "relay.footer_models")
                    }
                }
                crate::app::state::RelayPopupMode::OpenCodeDefaultModel
                | crate::app::state::RelayPopupMode::OpenCodeSmallModel => {
                    crate::i18n::t(locale, "relay.footer_model_picker")
                }
                crate::app::state::RelayPopupMode::None => {
                    crate::i18n::t(locale, "relay.footer_detail")
                }
            }
        } else if app
            .config
            .agents
            .get(app.relay_selected_agent)
            .map(|agent| agent.name.as_str() == "codex")
            .unwrap_or(false)
        {
            crate::i18n::t(locale, "relay.footer_detail_codex")
        } else if app
            .config
            .agents
            .get(app.relay_selected_agent)
            .map(|agent| agent.name.as_str() == "opencode")
            .unwrap_or(false)
        {
            crate::i18n::t(locale, "relay.footer_detail_opencode")
        } else {
            crate::i18n::t(locale, "relay.footer_detail")
        }
    }

    pub(super) fn yes_no(ready: bool) -> &'static str {
        if ready {
            "ready"
        } else {
            "missing"
        }
    }

    pub(super) fn http_status_color(status: u16, theme: &Theme) -> Color {
        match status {
            100..=399 => theme.success,
            400..=499 => theme.warning,
            500..=599 => theme.error,
            _ => theme.comment,
        }
    }

    pub(super) fn latency_color(latency_ms: u64, theme: &Theme) -> Color {
        match latency_ms {
            0..=800 => theme.success,
            801..=2500 => theme.warning,
            _ => theme.error,
        }
    }
}
pub(crate) mod list;
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
