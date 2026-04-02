use super::common::{fallback_color, pad_to_width, truncate_to_width};
use super::markdown::{
    detail_surface, flatten_lines_for_smooth_scrolling, markdown_options,
    normalize_session_detail_markdown, render_detail_content_line, render_detail_padding_line,
    render_detail_separator_line, total_span_count, wrap_text_to_width,
};
use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(crate) const SESSION_ITEM_CONTENT_HEIGHT: usize = 2;
pub(crate) const SESSION_ITEM_GAP_HEIGHT: usize = 1;

pub(crate) fn draw_session_preview(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    if app.preview.view == crate::model::PreviewView::SessionDetail {
        if let Some(selected) = app.preview.expanded_turn {
            draw_session_detail(f, app, area, theme, selected);
            return;
        }
    }
    if app.preview.view == crate::model::PreviewView::SessionList
        || app.preview.view == crate::model::PreviewView::SessionDetail
    {
        draw_session_list(f, app, area, theme);
    } else if let Some(selected) = app.preview.expanded_turn {
        draw_session_detail(f, app, area, theme, selected);
    } else {
        draw_session_list(f, app, area, theme);
    }
}

fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let width = area.width.max(8) as usize;
    let (lines, selected_range) =
        build_session_list_lines(&app.preview.turns, app.preview.selected_turn, width, theme);

    let scroll = resolve_session_list_scroll(app, selected_range, area.height, lines.len());
    let paragraph = Paragraph::new(ratatui::text::Text::from(lines))
        .style(Style::default().fg(theme.fg))
        .scroll((scroll, 0));
    f.render_widget(paragraph, area);
}

fn draw_session_detail(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme, selected: usize) {
    let Some(turn) = app.preview.turns.get(selected).cloned() else {
        return;
    };
    let target_key = app.preview.pane_id.clone().unwrap_or_default();
    let theme_name = theme.name.to_string();
    let cache = app.cached_preview_detail_for(
        &target_key,
        selected,
        area.width,
        &theme_name,
        &turn.question,
        &turn.answer,
    );

    if let Some(cache) = cache {
        let total_lines = cache.lines.len();
        let scroll = resolve_preview_scroll_for_line_count(app, total_lines, area.height);
        render_detail_viewport(f, area, &cache.lines, scroll, detail_surface(theme));
        return;
    }

    let loading = vec![
        Line::from(Span::styled(
            " Rendering markdown preview… ",
            Style::default()
                .fg(theme.comment)
                .bg(detail_surface(theme))
                .add_modifier(Modifier::DIM),
        )),
        Line::from(Span::styled(
            " Press j/k to keep browsing while it prepares. ",
            Style::default().fg(theme.comment).bg(detail_surface(theme)),
        )),
    ];
    render_detail_viewport(f, area, &loading, 0, detail_surface(theme));
}

fn render_detail_viewport(
    f: &mut Frame,
    area: Rect,
    lines: &[Line<'static>],
    scroll: u16,
    surface_bg: Color,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let window = visible_detail_window(lines.len(), scroll, area.height);
    let blank = " ".repeat(area.width as usize);
    let fill_style = Style::default().bg(surface_bg);
    let buf = f.buffer_mut();

    for row in 0..area.height as usize {
        let y = area.y + row as u16;
        if let Some(line) = lines.get(window.start + row) {
            buf.set_line(area.x, y, line, area.width);
        } else {
            buf.set_string(area.x, y, &blank, fill_style);
        }
    }
}

pub fn render_session_detail_lines(
    turn: &crate::model::PreviewTurn,
    width: u16,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let canvas_width = width.max(12) as usize;
    let options = markdown_options(theme);
    let surface_bg = detail_surface(theme);

    let prompt =
        normalize_session_detail_markdown(&question_text_for_display(turn.question.trim()));
    let response = normalize_session_detail_markdown(&answer_text_for_display(
        turn.answer.as_deref().unwrap_or("...").trim(),
    ));

    let prompt_text = tui_markdown::from_str_with_options(&prompt, &options);
    let response_text = tui_markdown::from_str_with_options(&response, &options);

    let prompt_width = canvas_width.saturating_sub(4).max(1);
    let response_width = canvas_width.saturating_sub(4).max(1);
    let prompt_lines = wrap_text_to_width(
        &prompt_text,
        prompt_width,
        fallback_color(theme.comment, theme.fg),
        surface_bg,
    );
    let response_lines = wrap_text_to_width(
        &response_text,
        response_width,
        fallback_color(theme.highlight_fg, theme.fg),
        surface_bg,
    );
    let response_span_count = total_span_count(&response_lines);
    let response_lines = if response_span_count >= super::DETAIL_SMOOTH_SPAN_THRESHOLD
        || response_lines.len() >= super::DETAIL_SMOOTH_LINE_THRESHOLD
    {
        crate::log_debug!(
            "preview.detail: smooth_response width={} lines={} spans={}",
            response_width,
            response_lines.len(),
            response_span_count
        );
        flatten_lines_for_smooth_scrolling(
            response_lines,
            fallback_color(theme.highlight_fg, theme.fg),
            surface_bg,
        )
    } else {
        response_lines
    };

    let mut lines = Vec::new();
    lines.push(render_detail_padding_line(canvas_width, surface_bg));
    for line in prompt_lines {
        lines.push(render_detail_content_line(line, prompt_width, surface_bg));
    }
    lines.push(render_detail_padding_line(canvas_width, surface_bg));
    lines.push(render_detail_separator_line(
        canvas_width,
        "Response",
        fallback_color(theme.bg, theme.highlight_bg),
        fallback_color(theme.border_focused, theme.accent),
        fallback_color(theme.border_focused, theme.accent),
        surface_bg,
    ));
    lines.push(render_detail_padding_line(canvas_width, surface_bg));
    for line in response_lines {
        lines.push(render_detail_content_line(line, response_width, surface_bg));
    }
    lines.push(render_detail_padding_line(canvas_width, surface_bg));
    lines
}

pub(crate) fn render_session_card(
    turn: &crate::model::PreviewTurn,
    selected: bool,
    width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    debug_assert_eq!(SESSION_ITEM_CONTENT_HEIGHT, 2);
    let inner_width = width.saturating_sub(6).max(6);
    let q = truncate_to_width(
        &question_text_for_display(turn.question.trim()),
        inner_width,
    );
    let a = truncate_to_width(
        &answer_text_for_display(turn.answer.as_deref().unwrap_or("...").trim()),
        inner_width,
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
        Style::default()
            .fg(theme.highlight_fg)
            .bg(block_bg)
            .add_modifier(Modifier::DIM)
    } else {
        Style::default()
            .fg(theme.comment)
            .bg(block_bg)
            .add_modifier(Modifier::DIM)
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
            Span::styled(a, text_style),
        ]),
    ]
}

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

fn render_session_gap_line(width: usize, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        " ".repeat(width.max(1)),
        Style::default().bg(theme.bg),
    ))
}

fn question_text_for_display(text: &str) -> String {
    strip_turn_prefix(text, &["Q:", "Q：", "Question:", "question:"]).to_string()
}

fn answer_text_for_display(text: &str) -> String {
    strip_turn_prefix(text, &["A:", "A：", "Answer:", "answer:"]).to_string()
}

pub(crate) fn localized_status_label(
    locale: crate::i18n::Locale,
    state: &crate::model::AgentState,
) -> &'static str {
    match state {
        crate::model::AgentState::Busy => crate::i18n::t(locale, "preview.working"),
        crate::model::AgentState::Waiting => crate::i18n::t(locale, "preview.waiting"),
        crate::model::AgentState::Idle => crate::i18n::t(locale, "preview.idle"),
    }
}

fn strip_turn_prefix<'a>(text: &'a str, prefixes: &[&str]) -> &'a str {
    let trimmed = text.trim();
    for prefix in prefixes {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.trim_start();
        }
    }
    trimmed
}

pub(crate) fn preview_badge(
    label: &str,
    fg: ratatui::style::Color,
    bg: ratatui::style::Color,
) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

pub(crate) fn preview_agent_badge_colors(
    agent_type: &crate::model::AgentType,
    theme: &Theme,
) -> (ratatui::style::Color, ratatui::style::Color) {
    match agent_type {
        crate::model::AgentType::Codex => (theme.bg, Color::Rgb(88, 166, 255)),
        crate::model::AgentType::Claude => (theme.bg, Color::Rgb(249, 140, 87)),
        crate::model::AgentType::Gemini => (theme.bg, Color::Rgb(180, 140, 255)),
        crate::model::AgentType::Kimi | crate::model::AgentType::OpenCode => {
            (Color::White, Color::Black)
        }
        crate::model::AgentType::Aider => (theme.bg, theme.success),
        crate::model::AgentType::Cursor => (theme.bg, Color::Rgb(180, 140, 255)),
        crate::model::AgentType::Unknown => (theme.fg, theme.comment),
    }
}

pub(crate) fn fixed_label(label: &str, width: usize, theme: &Theme) -> Span<'static> {
    Span::styled(
        format!("{} ", pad_to_width(label, width)),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::BOLD),
    )
}

pub(crate) fn resolve_session_list_scroll(
    app: &mut App,
    selected_range: Option<(usize, usize)>,
    viewport_height: u16,
    total_lines: usize,
) -> u16 {
    if viewport_height == 0 {
        return 0;
    }
    let max_scroll = total_lines.saturating_sub(viewport_height as usize);
    let mut scroll = app
        .preview
        .list_scroll
        .min(max_scroll.min(u16::MAX as usize) as u16);

    if app.preview.follow_selection {
        if let Some((start, end)) = selected_range {
            let scroll_usize = scroll as usize;
            let viewport = viewport_height as usize;
            if start < scroll_usize {
                scroll = start.min(max_scroll).min(u16::MAX as usize) as u16;
            } else if end >= scroll_usize.saturating_add(viewport) {
                let adjusted = end
                    .saturating_add(1)
                    .saturating_sub(viewport)
                    .min(max_scroll)
                    .min(u16::MAX as usize);
                scroll = adjusted as u16;
            }
        }
    }

    app.preview.list_scroll = scroll;
    scroll
}

pub(crate) fn resolve_preview_scroll_for_line_count(
    app: &mut App,
    total_lines: usize,
    viewport_height: u16,
) -> u16 {
    if viewport_height == 0 {
        return 0;
    }
    let max_scroll = total_lines.saturating_sub(viewport_height as usize);
    let max_scroll = max_scroll.min(u16::MAX as usize) as u16;
    let scroll = if app.preview_uses_detail_scroll() {
        app.preview.detail_scroll.min(max_scroll)
    } else if app.preview.follow_bottom {
        max_scroll
    } else {
        app.preview.scroll.min(max_scroll)
    };
    if app.preview_uses_detail_scroll() {
        app.preview.detail_scroll = scroll;
    } else {
        app.preview.scroll = scroll;
    }
    scroll
}

pub(crate) fn visible_detail_window(
    total_lines: usize,
    scroll: u16,
    viewport_height: u16,
) -> std::ops::Range<usize> {
    let start = scroll as usize;
    let end = start
        .saturating_add(viewport_height as usize)
        .min(total_lines);
    start..end
}

#[cfg(test)]
mod tests {
    use super::{build_session_list_lines, session_list_total_lines, session_turn_index_at_line};
    use crate::model::PreviewTurn;
    use crate::theme::Theme;

    #[test]
    fn selected_range_excludes_gap_line() {
        let turns = vec![
            PreviewTurn {
                question: "first".into(),
                answer: Some("one".into()),
            },
            PreviewTurn {
                question: "second".into(),
                answer: Some("two".into()),
            },
        ];

        let (lines, selected_range) =
            build_session_list_lines(&turns, Some(0), 40, &Theme::default());
        assert_eq!(lines.len(), 5);
        assert_eq!(selected_range, Some((0, 1)));
    }

    #[test]
    fn gap_line_has_no_turn_hit_target() {
        assert_eq!(session_list_total_lines(2), 5);
        assert_eq!(session_turn_index_at_line(0, 2), Some(0));
        assert_eq!(session_turn_index_at_line(1, 2), Some(0));
        assert_eq!(session_turn_index_at_line(2, 2), None);
        assert_eq!(session_turn_index_at_line(3, 2), Some(1));
        assert_eq!(session_turn_index_at_line(4, 2), Some(1));
        assert_eq!(session_turn_index_at_line(5, 2), None);
    }
}
