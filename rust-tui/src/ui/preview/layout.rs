use super::common::{display_width, truncate_to_width};
use super::session::{
    fixed_label, preview_agent_badge_colors, preview_badge, render_session_card,
    resolve_preview_scroll_for_line_count, resolve_session_list_scroll, visible_detail_window,
};
use crate::app::App;
use crate::sidebar::SidebarThread;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub(crate) fn draw_preview_info_card(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    thread: &SidebarThread,
) {
    let l = app.locale;
    let live_panel = app.selected_panel().cloned();
    let cache_badge_label = if app.preview.source == crate::model::PreviewSource::Session
        && app.preview.session_origin != Some(crate::model::PreviewSessionOrigin::App)
        && thread.session_cache_state == Some(crate::model::SessionCacheState::Cached)
    {
        Some(crate::i18n::t(l, "preview.session_cached"))
    } else {
        None
    };
    let status_label = super::session::localized_status_label(l, &thread.state);
    let status_color = match thread.state {
        crate::model::AgentState::Busy => theme.warning,
        crate::model::AgentState::Waiting => theme.success,
        crate::model::AgentState::Idle => theme.comment,
    };
    let branch = thread
        .git_info
        .as_ref()
        .and_then(|git| git.branch.as_ref())
        .map(|branch| branch.as_str());
    let git_text = if let Some(panel) = live_panel {
        panel.git_display()
    } else if let Some(git) = thread.git_info.as_ref() {
        let branch = git.branch.as_deref().unwrap_or("?");
        let commit = git.commit.as_deref().unwrap_or("?");
        format!("{}@{}", branch, truncate_to_width(commit, 7))
    } else {
        String::from("—")
    };
    let session_id = app
        .preview
        .session_id
        .as_deref()
        .or(thread.session_id.as_deref())
        .unwrap_or("—");
    let label_width = 6usize;
    let header = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.highlight_bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.border));
    let inner = header.inner(area);
    f.render_widget(header, area);
    let (agent_badge_fg, agent_badge_bg) = preview_agent_badge_colors(&thread.agent_type, theme);

    let mut badge_spans = vec![preview_badge(
        &thread.agent_type.to_string().to_uppercase(),
        agent_badge_fg,
        agent_badge_bg,
    )];
    badge_spans.push(Span::raw(" "));
    badge_spans.push(preview_badge(status_label, theme.bg, status_color));
    badge_spans.push(Span::raw(" "));
    badge_spans.push(preview_badge(
        &format!("PID {}", thread.pid.as_deref().unwrap_or("—")),
        theme.fg,
        theme.bg,
    ));
    if let Some(label) = cache_badge_label {
        badge_spans.push(Span::raw(" "));
        badge_spans.push(preview_badge(label, theme.bg, theme.warning));
    }
    if let Some(branch) = branch {
        badge_spans.push(Span::raw(" "));
        badge_spans.push(preview_badge(
            &truncate_to_width(branch, 16),
            theme.fg,
            theme.bg,
        ));
    }

    let card = vec![
        Line::from(badge_spans),
        Line::from(vec![
            fixed_label("LOC", label_width, theme),
            Span::styled(
                truncate_to_width(
                    thread.live_location.as_deref().unwrap_or("—"),
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("PATH", label_width, theme),
            Span::styled(
                truncate_to_width(
                    &shortened_thread_path(
                        thread,
                        inner.width.saturating_sub((label_width + 1) as u16) as usize,
                    ),
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("GIT", label_width, theme),
            Span::styled(
                truncate_to_width(
                    &git_text,
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("SID", label_width, theme),
            Span::styled(
                truncate_to_width(
                    session_id,
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
    ];

    let paragraph =
        Paragraph::new(card).style(Style::default().bg(theme.highlight_bg).fg(theme.fg));
    f.render_widget(paragraph, inner);
}

pub fn preview_sid_text_at(app: &mut App, area: Rect, column: u16, row: u16) -> Option<String> {
    let thread = app.selected_preview_thread()?;

    let session_id = app
        .preview
        .session_id
        .as_deref()
        .or(thread.session_id.as_deref())
        .unwrap_or("—");
    if session_id == "—" || area.width < 3 || area.height < 3 {
        return None;
    }

    let inner = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .inner(area);
    let sid_row = inner.y.saturating_add(4);
    if row != sid_row {
        return None;
    }

    let label_width = 6u16;
    let value_x = inner.x.saturating_add(label_width + 1);
    let max_width = inner.width.saturating_sub(label_width + 1) as usize;
    let visible = truncate_to_width(session_id, max_width);
    let value_width = display_width(&visible) as u16;

    if column >= value_x && column < value_x.saturating_add(value_width.max(1)) {
        Some(session_id.to_string())
    } else {
        None
    }
}

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

pub(crate) fn preview_visible_plain_text_rows(app: &mut App, area: Rect) -> Vec<String> {
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
    let target_key = app.preview.pane_id.clone().unwrap_or_default();
    let theme_name = app.theme.name.to_string();
    let content = app.preview.content.clone();
    let cache_hit = app.preview.plain_cache.as_ref().is_some_and(|cache| {
        cache.target_key == target_key
            && cache.width == area.width
            && cache.theme_name == theme_name
            && cache.content == content
    });

    if !cache_hit {
        let lines: Vec<Line<'static>> = content
            .lines()
            .map(|line| Line::from(super::markdown::format_line(line, &app.theme)))
            .collect();
        let wrapped_rows = super::plain::wrapped_row_count_for_lines(&lines, area.width as usize);
        app.preview.plain_cache = Some(crate::app::PreviewPlainCache {
            target_key,
            width: area.width,
            theme_name,
            content,
            lines,
            wrapped_rows,
        });
    }

    let scroll = super::plain::resolve_preview_scroll_from_cache(app, area) as usize;
    let wrapped = app
        .preview
        .plain_cache
        .as_ref()
        .map(|cache| {
            cache
                .lines
                .iter()
                .flat_map(|line| super::markdown::wrap_styled_line(line, area.width as usize))
                .map(|line| line_to_plain_string(&line))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    wrapped
        .into_iter()
        .skip(scroll)
        .take(area.height as usize)
        .collect()
}

fn preview_session_list_visible_rows(app: &mut App, area: Rect) -> Vec<String> {
    let width = area.width.max(8) as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut selected_range = None;

    for (idx, turn) in app.preview.turns.iter().enumerate() {
        let start = lines.len();
        lines.extend(render_session_card(
            turn,
            idx == app.preview.selected_turn.unwrap_or(usize::MAX),
            width,
            &app.theme,
        ));
        let end = lines.len().saturating_sub(1);
        if app.preview.selected_turn == Some(idx) {
            selected_range = Some((start, end));
        }
    }

    let scroll =
        resolve_session_list_scroll(app, selected_range, area.height, lines.len()) as usize;
    lines
        .into_iter()
        .skip(scroll)
        .take(area.height as usize)
        .map(|line| line_to_plain_string(&line))
        .collect()
}

fn preview_detail_visible_rows(app: &mut App, area: Rect) -> Vec<String> {
    let Some(selected) = app.preview.expanded_turn else {
        return Vec::new();
    };
    let Some(turn) = app.preview.turns.get(selected).cloned() else {
        return Vec::new();
    };

    let target_key = app.preview.pane_id.clone().unwrap_or_default();
    let theme_name = app.theme.name.to_string();
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
            super::session::render_session_detail_lines(&turn, area.width, &app.theme)
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
        let width = super::common::char_display_width(ch).max(1);
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

fn shortened_thread_path(thread: &SidebarThread, max_len: usize) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = if thread.working_dir.starts_with(&home) {
        thread.working_dir.replacen(&home, "~", 1)
    } else {
        thread.working_dir.clone()
    };

    if path.len() <= max_len {
        return path;
    }

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 {
        let short = format!(
            "~/.../{}/{}",
            parts[parts.len() - 2],
            parts[parts.len() - 1]
        );
        if short.len() <= max_len {
            return short;
        }
    }

    let start = path
        .char_indices()
        .rev()
        .find(|(i, _)| path.len() - i <= max_len.saturating_sub(3))
        .map(|(i, _)| i)
        .unwrap_or(0);
    format!("...{}", &path[start..])
}
