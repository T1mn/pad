mod lines;
pub(crate) mod opencode;
mod test_status {
    use super::super::layout::{http_status_color, latency_color};
    use crate::i18n::Locale;
    use crate::theme::Theme;
    use ratatui::{
        style::{Modifier, Style},
        text::{Line, Span},
    };

    pub(super) struct ProviderTestStatus<'a> {
        pub(super) in_progress: bool,
        pub(super) status: Option<bool>,
        pub(super) http_status: Option<u16>,
        pub(super) latency_ms: Option<u64>,
        pub(super) result: Option<&'a str>,
        pub(super) theme: &'a Theme,
        pub(super) locale: Locale,
    }

    pub(super) fn append_provider_test_lines(
        lines: &mut Vec<Line<'static>>,
        status: ProviderTestStatus<'_>,
    ) {
        if !status.in_progress && status.status.is_none() && status.result.is_none() {
            return;
        }

        lines.push(Line::from(""));

        if status.in_progress {
            lines.push(Line::from(Span::styled(
                crate::i18n::t(status.locale, "relay.testing"),
                Style::default()
                    .fg(status.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            return;
        }

        let (label, color) = match status.status {
            Some(true) => ("Reachable", status.theme.success),
            Some(false) => ("Test Failed", status.theme.error),
            None => ("Test", status.theme.comment),
        };
        let mut summary_spans = vec![Span::styled(
            label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )];
        if let Some(code) = status.http_status {
            summary_spans.push(Span::raw("  "));
            summary_spans.push(Span::styled(
                format!("HTTP {}", code),
                Style::default()
                    .fg(http_status_color(code, status.theme))
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if let Some(latency_ms) = status.latency_ms {
            summary_spans.push(Span::raw("  "));
            summary_spans.push(Span::styled(
                format!("{} ms", latency_ms),
                Style::default()
                    .fg(latency_color(latency_ms, status.theme))
                    .add_modifier(Modifier::BOLD),
            ));
        }
        lines.push(Line::from(summary_spans));

        if let Some(result) = status.result {
            for line in result.lines().take(4) {
                lines.push(Line::from(Span::styled(
                    super::super::super::common::truncate_modal_line(line, 72),
                    Style::default().fg(status.theme.comment),
                )));
            }
        }
    }
}

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
