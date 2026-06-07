use super::super::common::truncate_to_width;
use super::super::session_list_cache::{
    ensure_session_list_cache, selected_session_list_range, visible_session_list_lines,
};
use super::scroll::resolve_session_list_scroll;
use super::text::{answer_text_for_display, question_text_for_display, split_preview_card_lines};
use super::{SESSION_ITEM_CONTENT_HEIGHT, SESSION_ITEM_GAP_HEIGHT};
use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(super) fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let width = area.width.max(8);
    ensure_session_list_cache(app, width, theme);

    let total_lines = session_list_total_lines(app.preview.turns.len());
    let selected_range =
        selected_session_list_range(app.preview.selected_turn, app.preview.turns.len());
    let scroll = resolve_session_list_scroll(app, selected_range, area.height, total_lines);
    let lines = visible_session_list_lines(app, width as usize, theme, scroll, area.height);
    let paragraph =
        Paragraph::new(ratatui::text::Text::from(lines)).style(Style::default().fg(theme.fg));
    f.render_widget(paragraph, area);
}

pub(crate) fn render_session_card(
    turn: &crate::model::PreviewTurn,
    selected: bool,
    width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    debug_assert_eq!(SESSION_ITEM_CONTENT_HEIGHT, 3);
    let inner_width = width.saturating_sub(6).max(6);
    let q = truncate_to_width(
        &question_text_for_display(turn.question.trim()),
        inner_width,
    );
    let answer_lines = split_preview_card_lines(
        &answer_text_for_display(turn.answer.as_deref().unwrap_or("...").trim()),
        inner_width,
        2,
    );
    let block_bg = if selected {
        theme.highlight_bg
    } else {
        theme.bg
    };
    let marker_style = if selected {
        Style::default().fg(theme.border_focused).bg(block_bg)
    } else {
        Style::default().fg(theme.border).bg(block_bg)
    };
    let q_label_style = if selected {
        Style::default().fg(theme.highlight_fg).bg(block_bg)
    } else {
        Style::default().fg(theme.accent).bg(block_bg)
    };
    let a_label_style = if selected {
        Style::default().fg(theme.highlight_fg).bg(block_bg)
    } else {
        Style::default().fg(theme.success).bg(block_bg)
    };
    let text_style = if selected {
        Style::default().fg(theme.highlight_fg).bg(block_bg)
    } else {
        Style::default().fg(theme.fg).bg(block_bg)
    };
    vec![
        Line::from(vec![
            Span::styled("▌", marker_style),
            Span::styled(" Q ", q_label_style),
            Span::styled(q, text_style),
        ]),
        Line::from(vec![
            Span::styled("▌", marker_style),
            Span::styled(" A ", a_label_style),
            Span::styled(
                answer_lines.first().cloned().unwrap_or_default(),
                text_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("▌", marker_style),
            Span::styled("   ", Style::default().bg(block_bg)),
            Span::styled(answer_lines.get(1).cloned().unwrap_or_default(), text_style),
        ]),
    ]
}

#[cfg(test)]
pub(crate) fn build_session_list_lines(
    turns: &[crate::model::PreviewTurn],
    selected_turn: Option<usize>,
    width: usize,
    theme: &Theme,
) -> (Vec<Line<'static>>, Option<(usize, usize)>) {
    let mut lines = Vec::new();
    let mut selected_range = None;

    for (idx, turn) in turns.iter().enumerate() {
        let start = lines.len();
        lines.extend(render_session_card(
            turn,
            selected_turn == Some(idx),
            width,
            theme,
        ));
        let end = lines.len().saturating_sub(1);
        if selected_turn == Some(idx) {
            selected_range = Some((start, end));
        }
        if idx + 1 < turns.len() {
            lines.push(render_session_gap_line(width, theme));
        }
    }

    (lines, selected_range)
}

pub(crate) fn session_list_total_lines(turn_count: usize) -> usize {
    if turn_count == 0 {
        0
    } else {
        turn_count * SESSION_ITEM_CONTENT_HEIGHT + (turn_count - 1) * SESSION_ITEM_GAP_HEIGHT
    }
}

pub(crate) fn session_turn_index_at_line(line: usize, turn_count: usize) -> Option<usize> {
    if line >= session_list_total_lines(turn_count) {
        return None;
    }

    let stride = SESSION_ITEM_CONTENT_HEIGHT + SESSION_ITEM_GAP_HEIGHT;
    let index = line / stride;
    let offset = line % stride;
    if offset < SESSION_ITEM_CONTENT_HEIGHT {
        Some(index)
    } else {
        None
    }
}

pub(crate) fn render_session_gap_line(width: usize, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        " ".repeat(width.max(1)),
        Style::default().bg(theme.bg),
    ))
}
