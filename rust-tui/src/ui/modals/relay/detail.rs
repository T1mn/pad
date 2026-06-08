mod lines;
mod opencode;
mod test_status;

use super::layout::relay_detail_footer_text;
use super::popup::draw_relay_popup;
use crate::app::App;
use lines::relay_detail_lines;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

pub(super) fn draw_relay_detail_content(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;

    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let provider =
        selected_agent.and_then(|agent| agent.providers.get(app.relay_selected_provider));
    let provider_label = provider.map(|prov| prov.label.as_str()).unwrap_or("?");
    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(
                "{} / {}",
                crate::i18n::t(locale, "relay.details"),
                provider_label
            ),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        header_area,
    );

    if let (Some(agent), Some(provider)) = (selected_agent, provider) {
        let detail_lines = relay_detail_lines(app, agent, provider, theme, locale);
        let paragraph = Paragraph::new(detail_lines).wrap(Wrap { trim: false });
        f.render_widget(paragraph, body_area);
    } else {
        let paragraph = Paragraph::new(vec![Line::from(Span::styled(
            crate::i18n::t(locale, "relay.no_provider"),
            Style::default().fg(theme.comment),
        ))])
        .wrap(Wrap { trim: false });
        f.render_widget(paragraph, body_area);
    }

    let footer_text = if app.relay_editing {
        crate::i18n::t(locale, "relay.footer_edit")
    } else {
        relay_detail_footer_text(app, locale)
    };
    let footer = Paragraph::new(Line::from(Span::styled(
        footer_text.to_string(),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::DIM),
    )));
    f.render_widget(footer, footer_area);

    if app.relay_popup_mode != crate::app::state::RelayPopupMode::None {
        draw_relay_popup(f, app, area);
    }
}
