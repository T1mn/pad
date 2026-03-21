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

pub fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
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
        format!(" Preview: {}{} ", panel.pane_id, git_info)
    } else {
        String::from(" Preview ")
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
                "Welcome to pad",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Tmux Agent Panel Manager",
                Style::default().fg(theme.fg),
            )),
            Line::from(""),
            Line::from(Span::styled("Key bindings:", Style::default().fg(theme.warning))),
            Line::from(Span::styled("  j/k or ↑/↓  Navigate panels", Style::default().fg(theme.fg))),
            Line::from(Span::styled("  Enter        Attach to panel", Style::default().fg(theme.fg))),
            Line::from(Span::styled("  /            Search panels", Style::default().fg(theme.fg))),
            Line::from(Span::styled("  t            Toggle file tree", Style::default().fg(theme.fg))),
            Line::from(Span::styled("  c            Create session", Style::default().fg(theme.fg))),
            Line::from(Span::styled("  d            Delete panel", Style::default().fg(theme.fg))),
            Line::from(Span::styled("  ?            Help", Style::default().fg(theme.fg))),
            Line::from(Span::styled("  F1           Settings", Style::default().fg(theme.fg))),
            Line::from(Span::styled("  q            Quit", Style::default().fg(theme.fg))),
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
            let info_spans = vec![
                Span::styled(
                    format!(" {} {} ", panel.agent_type.emoji(), panel.agent_type),
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled("│ ", Style::default().fg(theme.comment)),
                Span::styled(
                    panel.status_icon(),
                    if panel.is_active {
                        Style::default().fg(theme.success)
                    } else {
                        Style::default().fg(theme.fg)
                    },
                ),
                Span::styled(
                    if panel.is_active { " active" } else { " idle" },
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
        let lines: Vec<Line> = app
            .preview_content
            .lines()
            .map(|line| Line::from(format_line(line, theme)))
            .collect();

        let paragraph = Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0));

        f.render_widget(paragraph, split[1]);
    } else {
        let lines: Vec<Line> = app
            .preview_content
            .lines()
            .map(|line| Line::from(format_line(line, theme)))
            .collect();

        let paragraph = Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0));

        f.render_widget(paragraph, area);
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

pub fn draw_file_preview(f: &mut Frame, app: &App, area: Rect) {
    use crate::tree::PreviewType;

    let theme = &app.theme;
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
        String::from(" File Preview ")
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));

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
