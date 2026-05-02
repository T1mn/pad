use pulldown_cmark::HeadingLevel;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

pub fn heading_style(level: u8) -> Style {
    match level {
        1 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        2 => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        3 => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        _ => Style::default()
            .fg(Color::Gray)
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

pub fn is_blank_line(line: Option<&Line<'_>>) -> bool {
    let Some(line) = line else { return true };
    line.spans.iter().all(|span| span.content.trim().is_empty())
}
