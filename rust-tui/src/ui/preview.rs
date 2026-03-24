use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
    layout::Rect,
};

pub fn draw_preview(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme.clone();
    let l = app.locale;
    let title = if let Some(panel) = app.selected_panel() {
        let git_info = if let Some(git) = &panel.git_info {
            if let Some(branch) = &git.branch {
                format!(" [{}]", branch)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        format!(" {}: {}{} ", crate::i18n::t(l, "preview.title"), panel.pane_id, git_info)
    } else {
        format!(" {} ", crate::i18n::t(l, "preview.title"))
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));

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
            Line::from(Span::styled(crate::i18n::t(l, "preview.keybindings"), Style::default().fg(theme.warning))),
            Line::from(Span::styled(crate::i18n::t(l, "preview.nav_panels"), Style::default().fg(theme.fg))),
            Line::from(Span::styled(crate::i18n::t(l, "preview.attach"), Style::default().fg(theme.fg))),
            Line::from(Span::styled(crate::i18n::t(l, "preview.search"), Style::default().fg(theme.fg))),
            Line::from(Span::styled(crate::i18n::t(l, "preview.tree"), Style::default().fg(theme.fg))),
            Line::from(Span::styled(crate::i18n::t(l, "preview.create"), Style::default().fg(theme.fg))),
            Line::from(Span::styled(crate::i18n::t(l, "preview.delete"), Style::default().fg(theme.fg))),
            Line::from(Span::styled(crate::i18n::t(l, "preview.help"), Style::default().fg(theme.fg))),
            Line::from(Span::styled(crate::i18n::t(l, "preview.settings"), Style::default().fg(theme.fg))),
            Line::from(Span::styled(crate::i18n::t(l, "preview.quit"), Style::default().fg(theme.fg))),
        ];
        let paragraph = Paragraph::new(welcome)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
        return;
    }

    // Split area: agent info bar (1 line) + preview content
    let has_panel = app.selected_panel().is_some();
    if has_panel {
        let inner = block.inner(area);
        f.render_widget(block, area);

        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Min(0)])
            .split(inner);

        // Agent info bar
        if let Some(panel) = app.selected_panel() {
            let uptime = panel.uptime_display();
            let pid_str = panel.pid.as_deref().unwrap_or("?");
            let status_label = match panel.state {
                crate::model::AgentState::Busy => crate::i18n::t(l, "preview.working"),
                crate::model::AgentState::Waiting => crate::i18n::t(l, "preview.waiting"),
                crate::model::AgentState::Idle => "",
            };
            let status_style = match panel.state {
                crate::model::AgentState::Busy => Style::default().fg(theme.warning),
                crate::model::AgentState::Waiting => Style::default().fg(theme.success),
                crate::model::AgentState::Idle => Style::default().fg(theme.comment),
            };
            let info_spans = vec![
                Span::styled(
                    format!(" {} {} ", panel.agent_type.emoji(), panel.agent_type),
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled("│ ", Style::default().fg(theme.comment)),
                Span::styled(
                    panel.status_icon(app.busy_animation_frame),
                    status_style,
                ),
                Span::styled(
                    if status_label.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", status_label)
                    },
                    Style::default().fg(theme.fg),
                ),
                Span::styled(" │ ", Style::default().fg(theme.comment)),
                Span::styled(format!("PID {}", pid_str), Style::default().fg(theme.fg)),
                Span::styled(" │ ", Style::default().fg(theme.comment)),
                Span::styled(format!("⏱ {}", uptime), Style::default().fg(theme.warning)),
            ];
            let info_line = Paragraph::new(Line::from(info_spans))
                .style(Style::default().bg(theme.highlight_bg));
            f.render_widget(info_line, split[0]);
        }

        // Preview content
        let scroll = resolve_preview_scroll(app, split[1]);
        let lines: Vec<Line> = app
            .preview_content
            .lines()
            .map(|line| Line::from(format_line(line, &theme)))
            .collect();

        let paragraph = Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        f.render_widget(paragraph, split[1]);
    } else {
        let inner = block.inner(area);
        let scroll = resolve_preview_scroll(app, inner);
        let lines: Vec<Line> = app
            .preview_content
            .lines()
            .map(|line| Line::from(format_line(line, &theme)))
            .collect();

        let paragraph = Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        f.render_widget(paragraph, area);
    }
}

fn resolve_preview_scroll(app: &mut App, viewport: Rect) -> u16 {
    let max_scroll = precise_preview_max_scroll(&app.preview_content, viewport.width, viewport.height);
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

    if total == 0 { 1 } else { total }
}

fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
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
    use super::precise_preview_max_scroll;

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
        "fn", "let", "mut", "if", "else", "for", "while", "match", "struct", "enum", "impl",
        "pub", "use", "mod", "const", "return", "true", "false", "None", "Some", "Ok", "Err",
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
