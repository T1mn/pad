use super::super::common::{blend_color, fallback_color};
use crate::theme::Theme;
use ratatui::style::{Color, Modifier, Style};
use tui_markdown::{Options as MarkdownOptions, StyleSheet};

#[derive(Clone)]
pub(crate) struct PreviewMarkdownStyleSheet {
    theme: Theme,
}

impl PreviewMarkdownStyleSheet {
    pub(crate) fn new(theme: &Theme) -> Self {
        Self {
            theme: theme.clone(),
        }
    }
}

impl StyleSheet for PreviewMarkdownStyleSheet {
    fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::default()
                .fg(self.theme.keyword)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            2 => Style::default()
                .fg(self.theme.accent)
                .add_modifier(Modifier::BOLD),
            3 => Style::default()
                .fg(self.theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            _ => Style::default()
                .fg(self.theme.comment)
                .add_modifier(Modifier::ITALIC),
        }
    }

    fn code(&self) -> Style {
        inline_code_style(&self.theme)
    }

    fn link(&self) -> Style {
        Style::default()
            .fg(self.theme.accent)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn blockquote(&self) -> Style {
        Style::default().fg(self.theme.comment)
    }

    fn heading_meta(&self) -> Style {
        Style::default()
            .fg(self.theme.comment)
            .add_modifier(Modifier::DIM)
    }

    fn metadata_block(&self) -> Style {
        Style::default().fg(self.theme.comment)
    }
}

pub(crate) fn markdown_options(theme: &Theme) -> MarkdownOptions<PreviewMarkdownStyleSheet> {
    MarkdownOptions::new(PreviewMarkdownStyleSheet::new(theme))
}

pub(crate) fn inline_code_style(theme: &Theme) -> Style {
    Style::default()
        .fg(derived_inline_code_fg(theme))
        .bg(derived_inline_code_bg(theme))
}

fn derived_inline_code_bg(theme: &Theme) -> Color {
    let base = fallback_color(theme.bg, theme.highlight_bg);
    let surface = fallback_color(theme.highlight_bg, theme.border);
    blend_color(surface, base, 0.72)
}

fn derived_inline_code_fg(theme: &Theme) -> Color {
    let base = fallback_color(theme.fg, theme.highlight_fg);
    let accent = fallback_color(theme.accent, base);
    blend_color(accent, base, 0.28)
}
