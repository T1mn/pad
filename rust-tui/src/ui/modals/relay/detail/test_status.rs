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
