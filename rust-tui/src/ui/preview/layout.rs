use super::common::{display_width, truncate_to_width};
use super::session::{
    fixed_label, preview_agent_badge_colors, preview_badge, resolve_preview_scroll_for_line_count,
    resolve_session_list_scroll, session_list_total_lines, visible_detail_window,
};
use super::session_list_cache::{
    ensure_session_list_cache, selected_session_list_range, visible_session_list_lines,
};
use crate::app::App;
use crate::model::AgentType;
use crate::sidebar::SidebarThread;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

const PREVIEW_INFO_LABEL_WIDTH: usize = 8;

pub(crate) fn draw_preview_info_card(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    thread: &SidebarThread,
) {
    let l = app.locale;
    let live_panel = thread
        .live_pane_id
        .as_deref()
        .and_then(|pane_id| app.panels.iter().find(|panel| panel.pane_id == pane_id))
        .cloned();
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
    let label_width = PREVIEW_INFO_LABEL_WIDTH;
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
        Line::from(vec![
            fixed_label("PROV", label_width, theme),
            Span::styled(
                truncate_to_width(
                    &preview_provider_value(app, thread),
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("USAGE", label_width, theme),
            Span::styled(
                truncate_to_width(
                    &preview_usage_value(thread),
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("SHARE", label_width, theme),
            Span::styled(
                truncate_to_width(
                    thread.share_url.as_deref().unwrap_or("—"),
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("SUMMARY", label_width, theme),
            Span::styled(
                truncate_to_width(
                    if app.config.codex.title_summary {
                        thread.generated_title.as_deref().unwrap_or("—")
                    } else {
                        "—"
                    },
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
        .or(thread.session_id.as_deref())?;
    preview_info_value_text_at(area, column, row, 4, session_id)
}

pub fn preview_share_url_text_at(
    app: &mut App,
    area: Rect,
    column: u16,
    row: u16,
) -> Option<String> {
    let thread = app.selected_preview_thread()?;
    let share_url = thread.share_url.as_deref()?;
    preview_info_value_text_at(area, column, row, 7, share_url)
}

fn preview_info_value_text_at(
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
    super::plain::ensure_plain_preview_cache(app, area.width);

    let scroll = super::plain::resolve_preview_scroll_from_cache(app, area) as usize;
    app.preview
        .plain_cache
        .as_ref()
        .into_iter()
        .flat_map(|cache| cache.lines.iter())
        .flat_map(|line| super::markdown::wrap_styled_line(line, area.width as usize))
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

fn preview_usage_value(thread: &SidebarThread) -> String {
    match (thread.cost.as_deref(), thread.token_summary.as_deref()) {
        (Some(cost), Some(tokens)) => format!("{cost} · {tokens}"),
        (Some(cost), None) => cost.to_string(),
        (None, Some(tokens)) => tokens.to_string(),
        (None, None) => "—".to_string(),
    }
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

fn preview_provider_value(app: &App, thread: &SidebarThread) -> String {
    let agent_name = match thread.agent_type {
        AgentType::OpenCode => "opencode",
        _ => return agent_provider_value(app, &thread.agent_type.to_string(), thread),
    };
    agent_provider_value(app, agent_name, thread)
}

fn agent_provider_value(app: &App, agent_name: &str, thread: &SidebarThread) -> String {
    let Some(agent) = app
        .config
        .agents
        .iter()
        .find(|agent| agent.name == agent_name)
    else {
        return "—".to_string();
    };
    if let Some(session_provider_name) = thread
        .session_provider_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if let Some(provider) = agent.providers.iter().find(|provider| {
            provider_identity_for_thread(thread, provider) == session_provider_name
        }) {
            return format_provider_value(thread, provider);
        }
        return session_provider_name.to_string();
    }

    let Some(provider) = agent.active() else {
        return "—".to_string();
    };
    format_provider_value(thread, provider)
}

fn provider_identity_for_thread(
    thread: &SidebarThread,
    provider: &crate::theme::ProviderConfig,
) -> String {
    match thread.agent_type {
        AgentType::Codex => provider.codex_provider_name(),
        AgentType::OpenCode => provider.opencode_provider_key().to_string(),
        _ => provider.label.trim().to_string(),
    }
}

fn format_provider_value(
    thread: &SidebarThread,
    provider: &crate::theme::ProviderConfig,
) -> String {
    let label = provider.label.trim();
    let url = match thread.agent_type {
        AgentType::Codex => provider.codex_base_url(),
        _ => provider.base_url.trim().trim_end_matches('/').to_string(),
    };

    match (label.is_empty(), url.is_empty()) {
        (true, true) => "—".to_string(),
        (false, true) => label.to_string(),
        (true, false) => url,
        (false, false) => format!("{label} · {url}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        preview_info_value_text_at, preview_provider_value, preview_visible_plain_text_rows,
    };
    use crate::app::App;
    use crate::model::{AgentState, AgentType, PreviewSource, PreviewView};
    use crate::sidebar::SidebarThread;
    use crate::theme::ProviderConfig;
    use ratatui::layout::Rect;

    #[test]
    fn preview_plain_visible_rows_respects_scroll_window_after_wrapping() {
        let mut app = App::new();
        app.preview.source = PreviewSource::Tmux;
        app.preview.view = PreviewView::Plain;
        app.preview.pane_id = Some("%1".into());
        app.preview.content = "abcd\nefgh".into();
        app.preview.follow_bottom = false;
        app.preview.scroll = 1;

        let rows = preview_visible_plain_text_rows(&mut app, Rect::new(0, 0, 2, 2));

        assert_eq!(rows, vec!["cd".to_string(), "ef".to_string()]);
        assert!(app.preview.plain_cache.is_some());
    }

    #[test]
    fn preview_info_value_hit_test_returns_full_truncated_value() {
        let area = Rect::new(0, 0, 24, 11);
        let value = "https://opencode.ai/s/very-long-share-id";

        let copied = preview_info_value_text_at(area, 10, 8, 7, value);

        assert_eq!(copied.as_deref(), Some(value));
        assert_eq!(preview_info_value_text_at(area, 2, 8, 7, value), None);
        assert_eq!(preview_info_value_text_at(area, 10, 5, 7, value), None);
    }

    #[test]
    fn preview_provider_value_prefers_session_bound_provider() {
        let mut app = App::new();
        if let Some(agent) = app
            .config
            .agents
            .iter_mut()
            .find(|agent| agent.name == "codex")
        {
            agent.providers = vec![
                ProviderConfig {
                    label: "relay-a".into(),
                    base_url: "http://127.0.0.1:8317".into(),
                    api_key: String::new(),
                    env_key: String::new(),
                    wire_api: "responses".into(),
                    provider_key: String::new(),
                    npm_package: String::new(),
                    models: Vec::new(),
                    test_status: None,
                    test_http_status: None,
                    test_latency_ms: None,
                    test_result: None,
                },
                ProviderConfig {
                    label: "relay-b".into(),
                    base_url: "http://127.0.0.1:8418".into(),
                    api_key: String::new(),
                    env_key: String::new(),
                    wire_api: "responses".into(),
                    provider_key: String::new(),
                    npm_package: String::new(),
                    models: Vec::new(),
                    test_status: None,
                    test_http_status: None,
                    test_latency_ms: None,
                    test_result: None,
                },
            ];
            agent.active_provider = Some(1);
        }

        let thread = SidebarThread {
            key: "codex:sid-1".into(),
            folder_key: "/repo".into(),
            working_dir: "/repo".into(),
            folder_label: "repo".into(),
            agent_type: AgentType::Codex,
            runtime_source: None,
            session_id: Some("sid-1".into()),
            transcript_path: None,
            session_provider_name: Some("relay_a".into()),
            title: "title".into(),
            upstream_title: None,
            generated_title: None,
            subtitle: None,
            title_override: None,
            note: None,
            share_url: None,
            cost: None,
            token_summary: None,
            tags: Vec::new(),
            pinned: false,
            updated_at: 0,
            sort_updated_at: 0,
            live_pane_id: None,
            live_location: None,
            pid: None,
            git_info: None,
            state: AgentState::Idle,
            is_active: false,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
            archived: false,
            deleted: false,
        };

        assert_eq!(
            preview_provider_value(&app, &thread),
            "relay-a · http://127.0.0.1:8317/v1"
        );
    }

    #[test]
    fn preview_provider_value_falls_back_to_active_provider_without_session_binding() {
        let mut app = App::new();
        if let Some(agent) = app
            .config
            .agents
            .iter_mut()
            .find(|agent| agent.name == "codex")
        {
            agent.providers = vec![ProviderConfig {
                label: "relay-a".into(),
                base_url: "http://127.0.0.1:8317".into(),
                api_key: String::new(),
                env_key: String::new(),
                wire_api: "responses".into(),
                provider_key: String::new(),
                npm_package: String::new(),
                models: Vec::new(),
                test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            }];
            agent.active_provider = Some(0);
        }

        let thread = SidebarThread {
            key: "codex:sid-1".into(),
            folder_key: "/repo".into(),
            working_dir: "/repo".into(),
            folder_label: "repo".into(),
            agent_type: AgentType::Codex,
            runtime_source: None,
            session_id: Some("sid-1".into()),
            transcript_path: None,
            session_provider_name: None,
            title: "title".into(),
            upstream_title: None,
            generated_title: None,
            subtitle: None,
            title_override: None,
            note: None,
            share_url: None,
            cost: None,
            token_summary: None,
            tags: Vec::new(),
            pinned: false,
            updated_at: 0,
            sort_updated_at: 0,
            live_pane_id: None,
            live_location: None,
            pid: None,
            git_info: None,
            state: AgentState::Idle,
            is_active: false,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
            archived: false,
            deleted: false,
        };

        assert_eq!(
            preview_provider_value(&app, &thread),
            "relay-a · http://127.0.0.1:8317/v1"
        );
    }
}
