use super::render::CodeBlockLanguage;
use pulldown_cmark::HeadingLevel;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

const HEADING_H1: Color = Color::Rgb(122, 162, 247);
const HEADING_H2: Color = Color::Rgb(187, 154, 247);
const HEADING_H3: Color = Color::Rgb(125, 207, 255);
const HEADING_MUTED: Color = Color::Rgb(192, 202, 245);
const CODE_BG: Color = Color::Rgb(26, 27, 38);
const CODE_FG: Color = Color::Rgb(192, 202, 245);
const CODE_PREFIX: Color = Color::Rgb(86, 95, 137);
const INLINE_CODE_BG: Color = Color::Rgb(42, 43, 61);
const INLINE_CODE_FG: Color = Color::Rgb(224, 175, 104);

pub fn heading_style(level: u8) -> Style {
    match level {
        1 => Style::default()
            .fg(HEADING_H1)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        2 => Style::default().fg(HEADING_H2).add_modifier(Modifier::BOLD),
        3 => Style::default().fg(HEADING_H3).add_modifier(Modifier::BOLD),
        _ => Style::default()
            .fg(HEADING_MUTED)
            .add_modifier(Modifier::BOLD),
    }
}

pub fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub fn code_block_style(language: Option<CodeBlockLanguage>) -> Style {
    Style::default()
        .fg(code_language_color(language))
        .bg(CODE_BG)
}

pub fn code_block_prefix_style(language: Option<CodeBlockLanguage>) -> Style {
    let fg = if language.is_some() {
        code_language_color(language)
    } else {
        CODE_PREFIX
    };
    Style::default().fg(fg).bg(CODE_BG)
}

fn code_language_color(language: Option<CodeBlockLanguage>) -> Color {
    match language {
        Some(CodeBlockLanguage::Shell) => Color::Rgb(158, 206, 106),
        Some(CodeBlockLanguage::Rust) => Color::Rgb(255, 158, 100),
        Some(CodeBlockLanguage::JavaScript) => Color::Rgb(224, 175, 104),
        Some(CodeBlockLanguage::TypeScript) => Color::Rgb(125, 207, 255),
        Some(CodeBlockLanguage::Python) => Color::Rgb(122, 162, 247),
        Some(CodeBlockLanguage::Data) => Color::Rgb(187, 154, 247),
        Some(CodeBlockLanguage::Markdown) => Color::Rgb(115, 218, 202),
        None => CODE_FG,
    }
}

pub fn inline_code_style() -> Style {
    Style::default()
        .fg(INLINE_CODE_FG)
        .bg(INLINE_CODE_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn is_blank_line(line: Option<&Line<'_>>) -> bool {
    let Some(line) = line else { return true };
    line.spans.iter().all(|span| span.content.trim().is_empty())
}
