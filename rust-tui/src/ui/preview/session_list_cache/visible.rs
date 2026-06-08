use crate::app::{App, PreviewSessionListCache};
use crate::theme::Theme;
use crate::ui::preview::session::{
    render_session_gap_line, session_list_total_lines, SESSION_ITEM_CONTENT_HEIGHT,
    SESSION_ITEM_GAP_HEIGHT,
};
use ratatui::text::Line;

pub(crate) fn visible_session_list_lines(
    app: &App,
    width: usize,
    theme: &Theme,
    scroll: u16,
    height: u16,
) -> Vec<Line<'static>> {
    let Some(cache) = app.preview.session_list_cache.as_ref() else {
        return Vec::new();
    };
    let start = scroll as usize;
    let end = start.saturating_add(height as usize);
    (start..end)
        .filter_map(|line_index| {
            session_list_cached_line(cache, app.preview.selected_turn, line_index, width, theme)
        })
        .collect()
}

fn session_list_cached_line(
    cache: &PreviewSessionListCache,
    selected_turn: Option<usize>,
    line_index: usize,
    width: usize,
    theme: &Theme,
) -> Option<Line<'static>> {
    if line_index >= session_list_total_lines(cache.items.len()) {
        return None;
    }
    let stride = SESSION_ITEM_CONTENT_HEIGHT + SESSION_ITEM_GAP_HEIGHT;
    let turn_index = line_index / stride;
    let offset = line_index % stride;
    if offset >= SESSION_ITEM_CONTENT_HEIGHT {
        return Some(render_session_gap_line(width, theme));
    }
    let item = cache.items.get(turn_index)?;
    let lines = if selected_turn == Some(turn_index) {
        &item.selected_lines
    } else {
        &item.normal_lines
    };
    lines.get(offset).cloned()
}
