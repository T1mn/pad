use super::super::super::common::truncate_to_width;
use super::super::text::{
    answer_text_for_display, question_text_for_display, split_preview_card_lines,
};
use super::super::SESSION_ITEM_CONTENT_HEIGHT;
use crate::theme::Theme;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

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

pub(crate) fn render_session_gap_line(width: usize, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        " ".repeat(width.max(1)),
        Style::default().bg(theme.bg),
    ))
}
