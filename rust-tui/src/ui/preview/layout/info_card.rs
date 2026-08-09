mod hit {
    use super::super::super::common::{display_width, truncate_to_width};
    use super::PREVIEW_INFO_LABEL_WIDTH;
    use crate::app::App;
    use ratatui::{
        layout::Rect,
        widgets::{Block, BorderType, Borders},
    };

    pub(super) fn preview_sid_text_at(
        app: &mut App,
        area: Rect,
        column: u16,
        row: u16,
    ) -> Option<String> {
        let thread = app.selected_preview_thread()?;
        let session_id = app
            .preview
            .session_id
            .as_deref()
            .or(thread.session_id.as_deref())?;
        preview_info_value_text_at(area, column, row, 3, session_id)
    }

    pub(super) fn preview_share_url_text_at(
        app: &mut App,
        area: Rect,
        column: u16,
        row: u16,
    ) -> Option<String> {
        let thread = app.selected_preview_thread()?;
        let share_url = thread.share_url.as_deref()?;
        preview_info_value_text_at(area, column, row, 6, share_url)
    }

    pub(in crate::ui::preview::layout) fn preview_info_value_text_at(
        area: Rect,
        column: u16,
        row: u16,
        line_offset: u16,
        value: &str,
    ) -> Option<String> {
        let value = value.trim();
        if value.is_empty() || value == "—" || area.width < 3 || area.height < 3 {
            return None;
        }

        let inner = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .inner(area);
        let target_row = inner.y.saturating_add(line_offset);
        if row != target_row {
            return None;
        }

        let label_width = PREVIEW_INFO_LABEL_WIDTH as u16;
        let value_x = inner.x.saturating_add(label_width + 1);
        let max_width = inner.width.saturating_sub(label_width + 1) as usize;
        let visible = truncate_to_width(value, max_width);
        let value_width = display_width(&visible) as u16;

        if column >= value_x && column < value_x.saturating_add(value_width.max(1)) {
            Some(value.to_string())
        } else {
            None
        }
    }
}
mod render {
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
        let (agent_badge_fg, agent_badge_bg) =
            preview_agent_badge_colors(&thread.agent_type, theme);

        let mut spans = vec![preview_badge(
            &thread.agent_type.to_string().to_uppercase(),
            agent_badge_fg,
            agent_badge_bg,
        )];
        spans.push(Span::raw(" "));
        spans.push(preview_badge(values.status_label, theme.bg, status_color));
        if let Some(label) = values.cache_badge_label {
            spans.push(Span::raw(" "));
            spans.push(preview_badge(label, theme.bg, theme.warning));
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
}
pub(crate) mod values;

use crate::app::App;
use crate::sidebar::SidebarThread;
use crate::theme::Theme;
use ratatui::{layout::Rect, Frame};

use render::PREVIEW_INFO_LABEL_WIDTH;

pub(crate) fn draw_preview_info_card(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    thread: &SidebarThread,
) {
    let value_width = area
        .width
        .saturating_sub(2)
        .saturating_sub((PREVIEW_INFO_LABEL_WIDTH + 1) as u16) as usize;
    let values = values::build_info_card_values(app, thread, value_width);
    render::render_info_card(f, area, theme, thread, &values);
}

pub fn preview_sid_text_at(app: &mut App, area: Rect, column: u16, row: u16) -> Option<String> {
    hit::preview_sid_text_at(app, area, column, row)
}

pub fn preview_share_url_text_at(
    app: &mut App,
    area: Rect,
    column: u16,
    row: u16,
) -> Option<String> {
    hit::preview_share_url_text_at(app, area, column, row)
}

#[cfg(test)]
pub(super) use hit::preview_info_value_text_at;
