use ratatui::style::{Color, Modifier, Style};

pub(super) const SEPARATOR: &str = " │ ";
pub(super) const DELETE_BG: Color = Color::Rgb(52, 18, 18);
pub(super) const ADD_BG: Color = Color::Rgb(18, 52, 24);

pub(super) fn file_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn hunk_style() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn meta_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub(super) fn delete_style() -> Style {
    Style::default().fg(Color::Red).bg(DELETE_BG)
}

pub(super) fn add_style() -> Style {
    Style::default().fg(Color::Green).bg(ADD_BG)
}

pub(super) fn line_no(value: Option<usize>, width: usize) -> String {
    value
        .map(|value| format!("{value:>width$}"))
        .unwrap_or_else(|| " ".repeat(width))
}

pub(super) fn fit(value: &str, width: usize) -> String {
    let mut out = String::new();
    let mut len = 0usize;
    for ch in value.chars().take(width) {
        out.push(ch);
        len += 1;
    }
    if len < width {
        out.extend(std::iter::repeat_n(' ', width - len));
    }
    out
}
