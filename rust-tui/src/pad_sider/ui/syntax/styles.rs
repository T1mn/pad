use ratatui::style::{Color, Modifier, Style};

pub(super) const FG: Color = Color::Rgb(212, 212, 212);
pub(super) const COMMENT: Color = Color::Rgb(106, 153, 85);
pub(super) const STRING: Color = Color::Rgb(206, 145, 120);
pub(super) const NUMBER: Color = Color::Rgb(181, 206, 168);
pub(super) const KEYWORD: Color = Color::Rgb(197, 134, 192);
pub(super) const TYPE: Color = Color::Rgb(78, 201, 176);
pub(super) const FUNCTION: Color = Color::Rgb(220, 220, 170);
pub(super) const PROPERTY: Color = Color::Rgb(156, 220, 254);
pub(super) const TAG: Color = Color::Rgb(86, 156, 214);
pub(super) const WARNING: Color = Color::Rgb(255, 203, 107);

pub(super) fn default_style() -> Style {
    Style::default().fg(FG)
}

pub(super) fn comment_style() -> Style {
    Style::default().fg(COMMENT).add_modifier(Modifier::ITALIC)
}

pub(super) fn string_style() -> Style {
    Style::default().fg(STRING)
}

pub(super) fn number_style() -> Style {
    Style::default().fg(NUMBER)
}

pub(super) fn keyword_style() -> Style {
    Style::default().fg(KEYWORD)
}

pub(super) fn type_style() -> Style {
    Style::default().fg(TYPE)
}

pub(super) fn function_style() -> Style {
    Style::default().fg(FUNCTION)
}

pub(super) fn property_style() -> Style {
    Style::default().fg(PROPERTY)
}

pub(super) fn operator_style() -> Style {
    default_style()
}

pub(super) fn tag_style() -> Style {
    Style::default().fg(TAG)
}
