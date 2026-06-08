mod lines;

use super::super::markdown::detail_surface;
use super::scroll::{resolve_preview_scroll_for_line_count, visible_detail_window};
pub use lines::render_session_detail_lines;

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
