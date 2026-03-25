use crate::app::state::FocusTarget;
use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_preview(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme.clone();
    let l = app.locale;
    let preview_is_focused = app.preview_focus == FocusTarget::Preview;
    let focus_mark = if preview_is_focused { "●" } else { "○" };
    let title = format!(" {} {} ", focus_mark, crate::i18n::t(l, "preview.title"));

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(if preview_is_focused {
            theme.border_focused
        } else {
            theme.border
        }));

    // Empty state for preview
    if app.panels.is_empty() {
        let welcome = vec![
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.welcome"),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.subtitle"),
                Style::default().fg(theme.fg),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.keybindings"),
                Style::default().fg(theme.warning),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.nav_panels"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.attach"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.search"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.tree"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.create"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.delete"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.help"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.settings"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.quit"),
                Style::default().fg(theme.fg),
            )),
        ];
        let paragraph = Paragraph::new(welcome)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
        return;
    }

    let has_panel = app.selected_panel().is_some();
    if has_panel {
        let inner = block.inner(area);
        f.render_widget(block.clone(), area);

        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(8), Constraint::Min(0)])
            .split(inner);

        if let Some(panel) = app.selected_panel() {
            draw_preview_info_card(f, app, split[0], &theme, panel);
        }

        if app.preview_source == crate::model::PreviewSource::Session
            && !app.preview_turns.is_empty()
        {
            draw_session_preview(f, app, split[1], &theme);
        } else {
            draw_plain_preview(f, app, split[1], false, &block, &theme);
        }
    } else {
        draw_plain_preview(f, app, area, true, &block, &theme);
    }
}

fn draw_preview_info_card(
    f: &mut Frame,
    app: &App,
    area: Rect,
    theme: &Theme,
    panel: &crate::model::AgentPanel,
) {
    let l = app.locale;
    let preview_source_label = match app.preview_source {
        crate::model::PreviewSource::Tmux => crate::i18n::t(l, "preview.source_tmux"),
        crate::model::PreviewSource::Session => crate::i18n::t(l, "preview.source_session"),
    };
    let cache_badge_label = if app.preview_source == crate::model::PreviewSource::Session
        && panel.session_cache_state == Some(crate::model::SessionCacheState::Cached)
    {
        Some(crate::i18n::t(l, "preview.session_cached"))
    } else {
        None
    };
    let status_label = localized_status_label(l, &panel.state);
    let status_color = match panel.state {
        crate::model::AgentState::Busy => theme.warning,
        crate::model::AgentState::Waiting => theme.success,
        crate::model::AgentState::Idle => theme.comment,
    };
    let branch = panel
        .git_info
        .as_ref()
        .and_then(|git| git.branch.as_ref())
        .map(|branch| branch.as_str());
    let git_text = if panel.git_info.is_some() {
        panel.git_display()
    } else {
        String::from("—")
    };
    let session_id = panel.agent_session_id.as_deref().unwrap_or("—");
    let label_width = 6usize;
    let header = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.highlight_bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.border));
    let inner = header.inner(area);
    f.render_widget(header, area);

    let mut badge_spans = vec![
        preview_badge(preview_source_label, theme.bg, theme.accent),
        Span::raw(" "),
        preview_badge(status_label, theme.bg, status_color),
    ];
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
            Span::styled(
                format!(
                    " {} {}",
                    panel.agent_type.emoji(),
                    panel.agent_type.to_string().to_uppercase()
                ),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default().bg(theme.highlight_bg)),
            Span::styled(
                "PID ",
                Style::default()
                    .fg(theme.comment)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                panel.pid.as_deref().unwrap_or("?").to_string(),
                Style::default().fg(theme.fg),
            ),
            Span::styled("  ⏱ ", Style::default().fg(theme.comment)),
            Span::styled(panel.uptime_display(), Style::default().fg(theme.warning)),
        ]),
        Line::from(vec![
            fixed_label("LOC", label_width, theme),
            Span::styled(
                truncate_to_width(
                    &format!("{}:{}.{}", panel.session, panel.window_index, panel.pane),
                    inner.width.saturating_sub((label_width + 1) as u16) as usize,
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            fixed_label("PATH", label_width, theme),
            Span::styled(
                truncate_to_width(
                    &panel.shortened_path(
                        inner.width.saturating_sub((label_width + 1) as u16) as usize
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

fn draw_plain_preview(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    with_block: bool,
    block: &Block,
    theme: &Theme,
) {
    let viewport = if with_block { block.inner(area) } else { area };
    let scroll = resolve_preview_scroll(app, viewport);
    let lines: Vec<Line> = app
        .preview_content
        .lines()
        .map(|line| Line::from(format_line(line, theme)))
        .collect();

    let mut paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    if with_block {
        paragraph = paragraph.block(block.clone());
    }

    f.render_widget(paragraph, area);
}

fn draw_session_preview(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    if let Some(selected) = app.preview_expanded_turn {
        draw_session_detail(f, app, area, theme, selected);
    } else {
        draw_session_list(f, app, area, theme);
    }
}

fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let width = area.width.max(8) as usize;
    let mut lines: Vec<Line> = Vec::new();
    let mut selected_range = None;

    for (idx, turn) in app.preview_turns.iter().enumerate() {
        let start = lines.len();
        lines.extend(render_session_card(
            turn,
            idx == app.preview_selected_turn.unwrap_or(usize::MAX),
            width,
            theme,
        ));
        let end = lines.len().saturating_sub(1);
        if app.preview_selected_turn == Some(idx) {
            selected_range = Some((start, end));
        }
    }

    let scroll = resolve_session_list_scroll(app, selected_range, area.height, lines.len());
    let paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(theme.fg))
        .scroll((scroll, 0));
    f.render_widget(paragraph, area);
}

fn draw_session_detail(f: &mut Frame, app: &mut App, area: Rect, _theme: &Theme, selected: usize) {
    let Some(turn) = app.preview_turns.get(selected) else {
        return;
    };
    let question = question_text_for_display(turn.question.trim());
    let answer = answer_text_for_display(turn.answer.as_deref().unwrap_or("...").trim());
    let detail = format!("**Q**\n\n{}\n\n---\n\n**A**\n\n{}", question, answer);
    let scroll = resolve_preview_scroll_for_text(app, &detail, area);
    let text = tui_markdown::from_str(&detail);
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    f.render_widget(paragraph, area);
}

fn render_session_card(
    turn: &crate::model::PreviewTurn,
    selected: bool,
    width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
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
        Style::default()
            .fg(theme.highlight_fg)
            .bg(block_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.accent)
            .bg(block_bg)
            .add_modifier(Modifier::BOLD)
    };
    let a_label_style = if selected {
        Style::default()
            .fg(theme.highlight_fg)
            .bg(block_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.success)
            .bg(block_bg)
            .add_modifier(Modifier::BOLD)
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
            Span::styled(a, text_style),
        ]),
        Line::from(Span::styled(" ", Style::default().bg(block_bg))),
    ]
}

fn question_text_for_display(text: &str) -> String {
    normalize_turn_text_for_display(strip_turn_prefix(
        text,
        &["Q:", "Q：", "Question:", "question:"],
    ))
}

fn answer_text_for_display(text: &str) -> String {
    normalize_turn_text_for_display(strip_turn_prefix(
        text,
        &["A:", "A：", "Answer:", "answer:"],
    ))
}

fn localized_status_label(
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

fn normalize_turn_text_for_display(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_fenced_code = false;

    for (idx, line) in text.lines().enumerate() {
        if idx > 0 {
            out.push('\n');
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fenced_code = !in_fenced_code;
            out.push_str(line);
            continue;
        }

        if in_fenced_code {
            out.push_str(line);
            continue;
        }

        let indent_len = line.len().saturating_sub(trimmed.len());
        let indent = &line[..indent_len];
        let normalized = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "));

        if let Some(rest) = normalized {
            out.push_str(indent);
            out.push_str("• ");
            out.push_str(rest);
        } else {
            out.push_str(line);
        }
    }

    out
}

fn preview_badge(
    label: &str,
    fg: ratatui::style::Color,
    bg: ratatui::style::Color,
) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

fn fixed_label(label: &str, width: usize, theme: &Theme) -> Span<'static> {
    Span::styled(
        format!("{} ", pad_to_width(label, width)),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::BOLD),
    )
}

fn resolve_session_list_scroll(
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
        .preview_list_scroll
        .min(max_scroll.min(u16::MAX as usize) as u16);

    if app.preview_follow_selection {
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

    app.preview_list_scroll = scroll;
    scroll
}

fn resolve_preview_scroll(app: &mut App, viewport: Rect) -> u16 {
    let content = app.preview_content.clone();
    resolve_preview_scroll_for_text(app, &content, viewport)
}

fn resolve_preview_scroll_for_text(app: &mut App, content: &str, viewport: Rect) -> u16 {
    let max_scroll = precise_preview_max_scroll(content, viewport.width, viewport.height);
    let scroll = if app.preview_follow_bottom {
        max_scroll
    } else {
        app.preview_scroll.min(max_scroll)
    };
    app.preview_scroll = scroll;
    scroll
}

fn precise_preview_max_scroll(content: &str, viewport_width: u16, viewport_height: u16) -> u16 {
    if viewport_width == 0 || viewport_height == 0 {
        return 0;
    }

    let wrapped_rows = wrapped_row_count(content, viewport_width as usize);
    let max_scroll = wrapped_rows.saturating_sub(viewport_height as usize);
    max_scroll.min(u16::MAX as usize) as u16
}

fn wrapped_row_count(content: &str, viewport_width: usize) -> usize {
    if viewport_width == 0 {
        return 0;
    }

    let mut total = 0usize;
    for line in content.lines() {
        let width = display_width(line);
        let rows = if width == 0 {
            1
        } else {
            width.div_ceil(viewport_width)
        };
        total += rows.max(1);
    }

    if total == 0 {
        1
    } else {
        total
    }
}

fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let ellipsis = "…";
    let ellipsis_width = display_width(ellipsis);
    let target_width = max_width.saturating_sub(ellipsis_width);
    let mut result = String::new();
    let mut used = 0usize;

    for ch in text.chars() {
        let width = char_display_width(ch);
        if used + width > target_width {
            break;
        }
        result.push(ch);
        used += width;
    }

    result.push_str(ellipsis);
    result
}

fn pad_to_width(text: &str, target_width: usize) -> String {
    let width = display_width(text);
    if width >= target_width {
        return text.to_string();
    }

    let mut out = String::from(text);
    out.push_str(&" ".repeat(target_width - width));
    out
}

fn char_display_width(c: char) -> usize {
    if c == '\t' {
        return 4;
    }
    if c.is_control() {
        return 0;
    }

    let code = c as u32;
    if matches!(
        code,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x3FFFD
    ) {
        2
    } else {
        1
    }
}

fn format_line<'a>(line: &'a str, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let stripped = line.trim();

    let user_markers = ["$", "#", "❯", ">", "%"];
    for marker in &user_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            spans.push(Span::styled(
                *marker,
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {}", content),
                Style::default().fg(theme.success),
            ));
            return spans;
        }
    }

    let ai_markers = ["●", "•", "💫", "🤖", "🟣", "🔵", "🟢", "⚡"];
    for marker in &ai_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            spans.push(Span::styled(
                *marker,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {}", content),
                Style::default().fg(theme.accent),
            ));
            return spans;
        }
    }

    if stripped.to_lowercase().contains("error") || stripped.to_lowercase().contains("failed") {
        spans.push(Span::styled(line, Style::default().fg(theme.error)));
        return spans;
    }

    if stripped.to_lowercase().contains("success")
        || stripped.to_lowercase().contains("done")
        || stripped.contains("✓")
    {
        spans.push(Span::styled(line, Style::default().fg(theme.success)));
        return spans;
    }

    spans.push(Span::raw(line));
    spans
}

#[cfg(test)]
mod tests {
    use super::{
        answer_text_for_display, localized_status_label, precise_preview_max_scroll,
        question_text_for_display, resolve_session_list_scroll,
    };
    use crate::app::App;

    #[test]
    fn bottom_scroll_accounts_for_wrapped_lines() {
        let content = "12345\n67890";
        let scroll = precise_preview_max_scroll(content, 3, 2);
        assert_eq!(scroll, 2);
    }

    #[test]
    fn short_content_does_not_scroll_past_top() {
        let content = "hi";
        let scroll = precise_preview_max_scroll(content, 20, 5);
        assert_eq!(scroll, 0);
    }

    #[test]
    fn cjk_width_counts_as_double_width() {
        let content = "你好\n世界";
        let scroll = precise_preview_max_scroll(content, 3, 1);
        assert_eq!(scroll, 3);
    }

    #[test]
    fn session_list_scroll_follows_selection_when_enabled() {
        let mut app = App::new();
        app.preview_follow_selection = true;
        app.preview_list_scroll = 0;

        let scroll = resolve_session_list_scroll(&mut app, Some((8, 11)), 4, 20);
        assert_eq!(scroll, 8);
        assert_eq!(app.preview_list_scroll, 8);
    }

    #[test]
    fn session_list_scroll_preserves_manual_scroll_when_follow_disabled() {
        let mut app = App::new();
        app.preview_follow_selection = false;
        app.preview_list_scroll = 6;

        let scroll = resolve_session_list_scroll(&mut app, Some((0, 3)), 4, 20);
        assert_eq!(scroll, 6);
        assert_eq!(app.preview_list_scroll, 6);
    }

    #[test]
    fn preview_display_strips_duplicate_role_prefixes() {
        assert_eq!(question_text_for_display("Q: how are you?"), "how are you?");
        assert_eq!(answer_text_for_display("A：all good"), "all good");
    }

    #[test]
    fn preview_display_preserves_plain_turn_text() {
        assert_eq!(
            question_text_for_display("plain question"),
            "plain question"
        );
        assert_eq!(answer_text_for_display("plain answer"), "plain answer");
    }

    #[test]
    fn preview_display_converts_markdown_bullets_to_dots() {
        assert_eq!(
            answer_text_for_display("- one\n  - two\n* three"),
            "• one\n  • two\n• three"
        );
    }

    #[test]
    fn preview_display_preserves_code_fence_bullets() {
        assert_eq!(
            answer_text_for_display("```text\n- keep\n```\n- convert"),
            "```text\n- keep\n```\n• convert"
        );
    }

    #[test]
    fn idle_status_badge_is_localized() {
        assert_eq!(
            localized_status_label(crate::i18n::Locale::ZhCN, &crate::model::AgentState::Idle),
            "空闲"
        );
    }
}

pub fn draw_file_preview(f: &mut Frame, app: &App, area: Rect) {
    use crate::tree::PreviewType;

    let theme = &app.theme;
    let l = app.locale;
    let title = if let Some(ref path) = app.file_preview_path {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let preview_type = PreviewType::from_path(path);
        let type_icon = match preview_type {
            PreviewType::Text => "📄",
            PreviewType::Markdown => "📝",
            PreviewType::Image => "🖼️",
            PreviewType::Directory => "📁",
            PreviewType::Binary => "📦",
            PreviewType::Unknown => "❓",
        };

        format!(" {} {} ", type_icon, file_name)
    } else {
        format!(" {} ", crate::i18n::t(l, "preview.file_title"))
    };

    let border_color = if app.mode == crate::app::state::Mode::FilePreview {
        theme.border_focused
    } else {
        theme.border
    };
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    // Markdown rendering via tui-markdown
    if let Some(ref path) = app.file_preview_path {
        let preview_type = PreviewType::from_path(path);
        if preview_type == PreviewType::Markdown {
            let text = tui_markdown::from_str(&app.file_preview_content);
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((app.file_preview_scroll, 0));
            f.render_widget(paragraph, area);
            return;
        }
    }

    let content = &app.file_preview_content;
    let lines: Vec<Line> = content
        .lines()
        .map(|line| Line::from(format_file_preview_line(line, theme)))
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.file_preview_scroll, 0));

    f.render_widget(paragraph, area);
}

fn format_file_preview_line<'a>(line: &'a str, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let trimmed = line.trim_start();
    let indent = line.chars().count() - trimmed.chars().count();

    if indent > 0 {
        spans.push(Span::raw(" ".repeat(indent)));
    }

    let stripped = trimmed;

    if let Some(idx) = stripped.find("//") {
        spans.push(Span::raw(&stripped[..idx]));
        spans.push(Span::styled(
            &stripped[idx..],
            Style::default().fg(theme.comment),
        ));
        return spans;
    }
    if stripped.starts_with('#') {
        spans.push(Span::styled(stripped, Style::default().fg(theme.success)));
        return spans;
    }

    if stripped.contains('"') || stripped.contains('\'') {
        let mut in_string = false;
        let mut string_start = 0;

        for (i, c) in stripped.char_indices() {
            if c == '"' || c == '\'' {
                if !in_string {
                    if i > string_start {
                        spans.push(Span::raw(&stripped[string_start..i]));
                    }
                    in_string = true;
                    string_start = i;
                } else {
                    spans.push(Span::styled(
                        &stripped[string_start..=i],
                        Style::default().fg(theme.string_color),
                    ));
                    in_string = false;
                    string_start = i + 1;
                }
            }
        }

        if string_start < stripped.len() {
            if in_string {
                spans.push(Span::styled(
                    &stripped[string_start..],
                    Style::default().fg(theme.string_color),
                ));
            } else {
                spans.push(Span::raw(&stripped[string_start..]));
            }
        }

        if !spans.is_empty() {
            return spans;
        }
    }

    let keywords = [
        "fn", "let", "mut", "if", "else", "for", "while", "match", "struct", "enum", "impl", "pub",
        "use", "mod", "const", "return", "true", "false", "None", "Some", "Ok", "Err",
    ];
    for kw in &keywords {
        if stripped.starts_with(kw)
            && (stripped.len() == kw.len()
                || !stripped[kw.len()..].starts_with(char::is_alphanumeric))
        {
            spans.push(Span::styled(*kw, Style::default().fg(theme.keyword)));
            if stripped.len() > kw.len() {
                spans.push(Span::raw(&stripped[kw.len()..]));
            }
            return spans;
        }
    }

    if stripped
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        let end = stripped
            .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '_')
            .unwrap_or(stripped.len());
        spans.push(Span::styled(
            &stripped[..end],
            Style::default().fg(theme.number),
        ));
        if end < stripped.len() {
            spans.push(Span::raw(&stripped[end..]));
        }
        return spans;
    }

    spans.push(Span::raw(line));
    spans
}
