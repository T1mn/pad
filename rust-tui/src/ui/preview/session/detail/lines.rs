use super::super::super::common::fallback_color;
use super::super::super::markdown::{
    detail_surface, flatten_lines_for_smooth_scrolling, markdown_options,
    normalize_session_detail_markdown, render_detail_content_line, render_detail_padding_line,
    render_detail_separator_line, total_span_count, wrap_text_to_width,
};
use super::super::text::{answer_text_for_display, question_text_for_display};
use crate::theme::Theme;
use ratatui::text::Line;

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
    let response_lines = if response_span_count >= super::super::super::DETAIL_SMOOTH_SPAN_THRESHOLD
        || response_lines.len() >= super::super::super::DETAIL_SMOOTH_LINE_THRESHOLD
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
