use super::super::common::{display_width, truncate_to_width};
use super::super::session::{fixed_label, preview_agent_badge_colors, preview_badge};
use super::provider::preview_provider_value;
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
    let status_label = super::super::session::localized_status_label(l, &thread.state);
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

pub(super) fn preview_info_value_text_at(
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
