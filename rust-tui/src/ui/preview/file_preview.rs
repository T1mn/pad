mod syntax;

use super::markdown::markdown_options;
use crate::app::App;
use ratatui::{
    layout::Alignment,
    style::Style,
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use syntax::format_plain_file_preview;

pub fn draw_file_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use crate::tree::PreviewType;

    let theme = &app.theme;
    let l = app.locale;
    let title = if let Some(ref path) = app.preview.file_preview_path {
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

    if let Some(ref path) = app.preview.file_preview_path {
        let preview_type = PreviewType::from_path(path);
        if preview_type == PreviewType::Markdown {
            let options = markdown_options(theme);
            let text =
                tui_markdown::from_str_with_options(&app.preview.file_preview_content, &options);
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((app.preview.file_preview_scroll, 0));
            f.render_widget(paragraph, area);
            return;
        }
    }

    let lines = format_plain_file_preview(&app.preview.file_preview_content, theme);
    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.preview.file_preview_scroll, 0));

    f.render_widget(paragraph, area);
}
