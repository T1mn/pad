use super::super::layout::{http_status_color, latency_color};
use crate::i18n::Locale;
use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

#[allow(clippy::too_many_arguments)]
pub(super) fn append_provider_test_lines(
    lines: &mut Vec<Line<'static>>,
    in_progress: bool,
    status: Option<bool>,
    http_status: Option<u16>,
    latency_ms: Option<u64>,
    result: Option<&str>,
    theme: &Theme,
    locale: Locale,
) {
    if !in_progress && status.is_none() && result.is_none() {
        return;
    }

    lines.push(Line::from(""));

    if in_progress {
        lines.push(Line::from(Span::styled(
            crate::i18n::t(locale, "relay.testing"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
        return;
    }

    let (label, color) = match status {
        Some(true) => ("Reachable", theme.success),
        Some(false) => ("Test Failed", theme.error),
        None => ("Test", theme.comment),
    };
    let mut summary_spans = vec![Span::styled(
        label,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )];
    if let Some(code) = http_status {
        summary_spans.push(Span::raw("  "));
        summary_spans.push(Span::styled(
            format!("HTTP {}", code),
            Style::default()
                .fg(http_status_color(code, theme))
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(latency_ms) = latency_ms {
        summary_spans.push(Span::raw("  "));
        summary_spans.push(Span::styled(
            format!("{} ms", latency_ms),
            Style::default()
                .fg(latency_color(latency_ms, theme))
                .add_modifier(Modifier::BOLD),
        ));
    }
    lines.push(Line::from(summary_spans));

    if let Some(result) = result {
        for line in result.lines().take(4) {
            lines.push(Line::from(Span::styled(
                super::super::super::common::truncate_modal_line(line, 72),
                Style::default().fg(theme.comment),
            )));
        }
    }
}
