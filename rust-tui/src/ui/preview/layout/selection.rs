use super::super::session::{
    resolve_preview_scroll_for_line_count, resolve_session_list_scroll, session_list_total_lines,
    visible_detail_window,
};
use super::super::session_list_cache::{
    ensure_session_list_cache, selected_session_list_range, visible_session_list_lines,
};
use crate::app::App;
use ratatui::{layout::Rect, text::Line};

pub fn extract_preview_selection_text(
    app: &mut App,
    area: Rect,
    anchor: (u16, u16),
    current: (u16, u16),
) -> Option<String> {
    if area.width == 0 || area.height == 0 {
        return None;
    }

    let rows = preview_visible_plain_text_rows(app, area);
    if rows.is_empty() {
        return None;
    }

    let (start, end) = normalized_selection_points(area, anchor, current);
    if start == end {
        return None;
    }

    let start_row = start.1 as usize;
    let end_row = end.1 as usize;
    let mut parts = Vec::new();

    for row_idx in start_row..=end_row.min(rows.len().saturating_sub(1)) {
        let text = rows.get(row_idx).map(String::as_str).unwrap_or("");
        let piece = if start_row == end_row {
            slice_text_by_width(text, start.0 as usize, end.0 as usize + 1)
        } else if row_idx == start_row {
            slice_text_by_width(text, start.0 as usize, usize::MAX)
        } else if row_idx == end_row {
            slice_text_by_width(text, 0, end.0 as usize + 1)
        } else {
            text.to_string()
        };
        parts.push(piece);
    }

    let joined = parts.join("\n");
    if joined.trim().is_empty() {
        None
    } else {
        Some(joined)
    }
}

pub(super) fn preview_visible_plain_text_rows(app: &mut App, area: Rect) -> Vec<String> {
    if area.width == 0 || area.height == 0 {
        return Vec::new();
    }

    if app.preview.source == crate::model::PreviewSource::Session
        && !app.preview.turns.is_empty()
        && app.preview.view == crate::model::PreviewView::SessionDetail
    {
        return preview_detail_visible_rows(app, area);
    }

    if app.preview.source == crate::model::PreviewSource::Session
        && !app.preview.turns.is_empty()
        && app.preview.view == crate::model::PreviewView::SessionList
    {
        return preview_session_list_visible_rows(app, area);
    }

    preview_plain_visible_rows(app, area)
}

fn preview_plain_visible_rows(app: &mut App, area: Rect) -> Vec<String> {
    super::super::plain::ensure_plain_preview_cache(app, area.width);

    let scroll = super::super::plain::resolve_preview_scroll_from_cache(app, area) as usize;
    app.preview
        .plain_cache
        .as_ref()
        .into_iter()
        .flat_map(|cache| cache.lines.iter())
        .flat_map(|line| super::super::markdown::wrap_styled_line(line, area.width as usize))
        .skip(scroll)
        .take(area.height as usize)
        .map(|line| line_to_plain_string(&line))
        .collect()
}

fn preview_session_list_visible_rows(app: &mut App, area: Rect) -> Vec<String> {
    let width = area.width.max(8) as usize;
    let theme = app.theme.clone();
    ensure_session_list_cache(app, width as u16, &theme);

    let total_lines = session_list_total_lines(app.preview.turns.len());
    let selected_range =
        selected_session_list_range(app.preview.selected_turn, app.preview.turns.len());
    let scroll = resolve_session_list_scroll(app, selected_range, area.height, total_lines);
    visible_session_list_lines(app, width, &theme, scroll, area.height)
        .into_iter()
        .map(|line| line_to_plain_string(&line))
        .collect()
}

fn preview_detail_visible_rows(app: &mut App, area: Rect) -> Vec<String> {
    let Some(selected) = app.preview.expanded_turn else {
        return Vec::new();
    };

    let target_key = app.preview.pane_id.clone().unwrap_or_default();
    let theme_name = app.theme.name.to_string();
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
        let scroll = resolve_preview_scroll_for_line_count(app, total_lines, area.height) as usize;
        let window = visible_detail_window(total_lines, scroll as u16, area.height);
        return app
            .current_preview_detail_cache_for_current_turns(
                &target_key,
                selected,
                area.width,
                &theme_name,
            )
            .map(|cache| {
                cache.lines[window]
                    .iter()
                    .map(line_to_plain_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }

    let Some(turn) = app.preview.turns.get(selected).cloned() else {
        return Vec::new();
    };
    let lines = app
        .cached_preview_detail_for(
            &target_key,
            selected,
            area.width,
            &theme_name,
            &turn.question,
            &turn.answer,
        )
        .map(|cache| cache.lines)
        .unwrap_or_else(|| {
            super::super::session::render_session_detail_lines(&turn, area.width, &app.theme)
        });

    let scroll = resolve_preview_scroll_for_line_count(app, lines.len(), area.height) as usize;
    let window = visible_detail_window(lines.len(), scroll as u16, area.height);
    lines[window]
        .iter()
        .map(line_to_plain_string)
        .collect::<Vec<_>>()
}

fn normalized_selection_points(
    area: Rect,
    anchor: (u16, u16),
    current: (u16, u16),
) -> ((u16, u16), (u16, u16)) {
    let start = clamped_point_in_area(area, anchor);
    let end = clamped_point_in_area(area, current);
    if (start.1, start.0) <= (end.1, end.0) {
        (start, end)
    } else {
        (end, start)
    }
}

fn clamped_point_in_area(area: Rect, point: (u16, u16)) -> (u16, u16) {
    let max_x = area.width.saturating_sub(1);
    let max_y = area.height.saturating_sub(1);
    let x = point.0.saturating_sub(area.x).min(max_x);
    let y = point.1.saturating_sub(area.y).min(max_y);
    (x, y)
}

fn line_to_plain_string(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>()
}

fn slice_text_by_width(text: &str, start: usize, end: usize) -> String {
    if start >= end {
        return String::new();
    }

    let mut out = String::new();
    let mut offset = 0usize;
    for ch in text.chars() {
        let width = super::super::common::char_display_width(ch).max(1);
        let ch_start = offset;
        let ch_end = offset + width;
        if ch_end > start && ch_start < end {
            out.push(ch);
        }
        offset = ch_end;
        if offset >= end {
            break;
        }
    }
    out
}
