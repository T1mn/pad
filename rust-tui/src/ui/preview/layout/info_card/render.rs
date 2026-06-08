use super::super::super::common::truncate_to_width;
use super::super::super::session::{fixed_label, preview_agent_badge_colors, preview_badge};
use super::values::InfoCardValues;
use crate::sidebar::SidebarThread;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub(super) const PREVIEW_INFO_LABEL_WIDTH: usize = 8;

pub(super) fn render_info_card(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    thread: &SidebarThread,
    values: &InfoCardValues,
) {
    let header = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.highlight_bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.border));
    let inner = header.inner(area);
    f.render_widget(header, area);

    let label_width = PREVIEW_INFO_LABEL_WIDTH;
    let card = vec![
        Line::from(info_badge_spans(theme, thread, values)),
        info_value_line("LOC", &values.location, label_width, inner.width, theme),
        info_value_line("PATH", &values.path_text, label_width, inner.width, theme),
        info_value_line("GIT", &values.git_text, label_width, inner.width, theme),
        info_value_line("SID", &values.session_id, label_width, inner.width, theme),
        info_value_line(
            "PROV",
            &values.provider_text,
            label_width,
            inner.width,
            theme,
        ),
        info_value_line("USAGE", &values.usage_text, label_width, inner.width, theme),
        info_value_line("SHARE", &values.share_url, label_width, inner.width, theme),
        info_value_line("SUMMARY", &values.summary, label_width, inner.width, theme),
    ];

    let paragraph =
        Paragraph::new(card).style(Style::default().bg(theme.highlight_bg).fg(theme.fg));
    f.render_widget(paragraph, inner);
}

fn info_badge_spans(
    theme: &Theme,
    thread: &SidebarThread,
    values: &InfoCardValues,
) -> Vec<Span<'static>> {
    let status_color = match thread.state {
        crate::model::AgentState::Busy => theme.warning,
        crate::model::AgentState::Waiting => theme.success,
        crate::model::AgentState::Idle => theme.comment,
    };
    let (agent_badge_fg, agent_badge_bg) = preview_agent_badge_colors(&thread.agent_type, theme);

    let mut spans = vec![preview_badge(
        &thread.agent_type.to_string().to_uppercase(),
        agent_badge_fg,
        agent_badge_bg,
    )];
    spans.push(Span::raw(" "));
    spans.push(preview_badge(values.status_label, theme.bg, status_color));
    spans.push(Span::raw(" "));
    spans.push(preview_badge(
        &format!("PID {}", thread.pid.as_deref().unwrap_or("—")),
        theme.fg,
        theme.bg,
    ));
    if let Some(label) = values.cache_badge_label {
        spans.push(Span::raw(" "));
        spans.push(preview_badge(label, theme.bg, theme.warning));
    }
    if let Some(branch) = values.branch.as_deref() {
        spans.push(Span::raw(" "));
        spans.push(preview_badge(
            &truncate_to_width(branch, 16),
            theme.fg,
            theme.bg,
        ));
    }
    spans
}

fn info_value_line<'a>(
    label: &'static str,
    value: &'a str,
    label_width: usize,
    inner_width: u16,
    theme: &Theme,
) -> Line<'a> {
    Line::from(vec![
        fixed_label(label, label_width, theme),
        Span::styled(
            truncate_to_width(
                value,
                inner_width.saturating_sub((label_width + 1) as u16) as usize,
            ),
            Style::default().fg(theme.fg),
        ),
    ])
}
