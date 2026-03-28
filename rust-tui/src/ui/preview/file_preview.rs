use super::markdown::{markdown_options, retokenize_inline_code};
use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::Alignment,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_file_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    if let Some(ref path) = app.file_preview_path {
        let preview_type = PreviewType::from_path(path);
        if preview_type == PreviewType::Markdown {
            let options = markdown_options(theme);
            let text = tui_markdown::from_str_with_options(&app.file_preview_content, &options);
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

fn format_file_preview_line(line: &str, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let trimmed = line.trim_start();
    let indent = line.chars().count() - trimmed.chars().count();

    if indent > 0 {
        spans.push(Span::raw(" ".repeat(indent)));
    }

    let stripped = trimmed;

    if let Some(idx) = stripped.find("//") {
        spans.push(Span::raw(stripped[..idx].to_string()));
        spans.push(Span::styled(
            stripped[idx..].to_string(),
            Style::default().fg(theme.comment),
        ));
        return retokenize_inline_code(spans, theme);
    }
    if stripped.starts_with('#') {
        spans.push(Span::styled(
            stripped.to_string(),
            Style::default().fg(theme.success),
        ));
        return retokenize_inline_code(spans, theme);
    }

    if stripped.contains('"') || stripped.contains('\'') {
        let mut in_string = false;
        let mut string_start = 0;

        for (i, c) in stripped.char_indices() {
            if c == '"' || c == '\'' {
                if !in_string {
                    if i > string_start {
                        spans.push(Span::raw(stripped[string_start..i].to_string()));
                    }
                    in_string = true;
                    string_start = i;
                } else {
                    spans.push(Span::styled(
                        stripped[string_start..=i].to_string(),
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
                    stripped[string_start..].to_string(),
                    Style::default().fg(theme.string_color),
                ));
            } else {
                spans.push(Span::raw(stripped[string_start..].to_string()));
            }
        }

        if !spans.is_empty() {
            return retokenize_inline_code(spans, theme);
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
            spans.push(Span::styled(
                (*kw).to_string(),
                Style::default().fg(theme.keyword),
            ));
            if stripped.len() > kw.len() {
                spans.push(Span::raw(stripped[kw.len()..].to_string()));
            }
            return retokenize_inline_code(spans, theme);
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
            stripped[..end].to_string(),
            Style::default().fg(theme.number),
        ));
        if end < stripped.len() {
            spans.push(Span::raw(stripped[end..].to_string()));
        }
        return retokenize_inline_code(spans, theme);
    }

    super::markdown::tokenize_inline_code(line, Style::default(), theme)
}
