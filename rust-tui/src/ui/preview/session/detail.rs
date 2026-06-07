use super::super::common::fallback_color;
use super::super::markdown::{
    detail_surface, flatten_lines_for_smooth_scrolling, markdown_options,
    normalize_session_detail_markdown, render_detail_content_line, render_detail_padding_line,
    render_detail_separator_line, total_span_count, wrap_text_to_width,
};
use super::scroll::{resolve_preview_scroll_for_line_count, visible_detail_window};
use super::text::{answer_text_for_display, question_text_for_display};
use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    Frame,
};

pub(super) fn draw_session_detail(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    selected: usize,
) {
    let target_key = app.preview.pane_id.clone().unwrap_or_default();
    let theme_name = theme.name.to_string();
    if app.ensure_preview_detail_cache_for_current_turns(
        &target_key,
        selected,
        area.width,
        &theme_name,
    ) {
        let total_lines = app
            .current_preview_detail_cache_for_current_turns(
                &target_key,
                selected,
                area.width,
                &theme_name,
            )
            .map(|cache| cache.lines.len())
            .unwrap_or_default();
        let scroll = resolve_preview_scroll_for_line_count(app, total_lines, area.height);
        if let Some(cache) = app.current_preview_detail_cache_for_current_turns(
            &target_key,
            selected,
            area.width,
            &theme_name,
        ) {
            render_detail_viewport(f, area, &cache.lines, scroll, detail_surface(theme));
            return;
        }
    }

    let Some(turn) = app.preview.turns.get(selected).cloned() else {
        return;
    };
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
        fallback_color(theme.fg, theme.highlight_fg),
        surface_bg,
    );
    let response_lines = wrap_text_to_width(
        &response_text,
        response_width,
        fallback_color(theme.highlight_fg, theme.fg),
        surface_bg,
    );
    let response_span_count = total_span_count(&response_lines);
    let response_lines = if response_span_count >= super::super::DETAIL_SMOOTH_SPAN_THRESHOLD
        || response_lines.len() >= super::super::DETAIL_SMOOTH_LINE_THRESHOLD
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
