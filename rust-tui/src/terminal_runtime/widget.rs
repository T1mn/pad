use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};
use unicode_width::UnicodeWidthStr;

use super::{CursorShape, PaneFrame, TerminalCell, TerminalColor, TextAttributes};

pub struct TerminalPaneWidget<'a> {
    frame: &'a PaneFrame,
    focused: bool,
}

impl<'a> TerminalPaneWidget<'a> {
    pub fn new(frame: &'a PaneFrame) -> Self {
        Self {
            frame,
            focused: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl Widget for TerminalPaneWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let border_color = if self.focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let title = format!(" {} ", self.frame.metadata.label);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buffer);

        let columns = inner.width.min(self.frame.terminal.size.columns);
        let rows = inner.height.min(self.frame.terminal.size.rows);
        for row in 0..rows {
            for column in 0..columns {
                let Some(source) = self.frame.terminal.cell(column, row) else {
                    continue;
                };
                let Some(target) = buffer.cell_mut((inner.x + column, inner.y + row)) else {
                    continue;
                };
                let remaining = inner.width.saturating_sub(column);
                if source.symbol.width() > usize::from(remaining) {
                    // A clipped wide grapheme would physically overwrite the
                    // pane border while Ratatui's logical buffer still holds
                    // the border cell, preventing a later diff from repairing it.
                    target.set_symbol(" ");
                } else {
                    target.set_symbol(&source.symbol);
                }
                target.set_style(cell_style(source));

                if self.focused {
                    if let Some(cursor) = self
                        .frame
                        .terminal
                        .cursor
                        .filter(|cursor| cursor.column == column && cursor.row == row)
                    {
                        target.set_style(
                            Style::default().add_modifier(cursor_modifier(cursor.shape)),
                        );
                    }
                }
            }
        }
    }
}

fn cursor_modifier(shape: CursorShape) -> Modifier {
    match shape {
        CursorShape::Underline => Modifier::UNDERLINED,
        // Ratatui's buffer has no beam or hollow-box cursor primitive. Keep
        // these visible with the same high-contrast fallback as a block.
        CursorShape::Block | CursorShape::Beam | CursorShape::HollowBlock => Modifier::REVERSED,
    }
}

fn cell_style(cell: &TerminalCell) -> Style {
    let mut style = Style::default();
    if let Some(color) = foreground_color(cell.foreground) {
        style = style.fg(color);
    }
    if let Some(color) = background_color(cell.background) {
        style = style.bg(color);
    }
    style.add_modifier(attribute_modifiers(cell.attributes))
}

fn foreground_color(color: TerminalColor) -> Option<Color> {
    match color {
        TerminalColor::DefaultForeground => None,
        TerminalColor::DefaultBackground => Some(Color::Reset),
        TerminalColor::Indexed(index) => Some(Color::Indexed(index)),
        TerminalColor::Named(index) => Some(named_color(index)),
        TerminalColor::Rgb(red, green, blue) => Some(Color::Rgb(red, green, blue)),
    }
}

fn background_color(color: TerminalColor) -> Option<Color> {
    match color {
        TerminalColor::DefaultBackground => None,
        TerminalColor::DefaultForeground => Some(Color::Reset),
        TerminalColor::Indexed(index) => Some(Color::Indexed(index)),
        TerminalColor::Named(index) => Some(named_color(index)),
        TerminalColor::Rgb(red, green, blue) => Some(Color::Rgb(red, green, blue)),
    }
}

fn named_color(index: u16) -> Color {
    match index {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::White,
        // Alacritty's dim palette follows its standard palette entries.
        259..=266 => named_color(index - 259),
        _ => Color::Reset,
    }
}

fn attribute_modifiers(attributes: TextAttributes) -> Modifier {
    let mut modifiers = Modifier::empty();
    if attributes.bold {
        modifiers.insert(Modifier::BOLD);
    }
    if attributes.dim {
        modifiers.insert(Modifier::DIM);
    }
    if attributes.italic {
        modifiers.insert(Modifier::ITALIC);
    }
    if attributes.underline {
        modifiers.insert(Modifier::UNDERLINED);
    }
    if attributes.inverse {
        modifiers.insert(Modifier::REVERSED);
    }
    if attributes.hidden {
        modifiers.insert(Modifier::HIDDEN);
    }
    if attributes.strikeout {
        modifiers.insert(Modifier::CROSSED_OUT);
    }
    modifiers
}

#[cfg(test)]
#[path = "widget_tests.rs"]
pub(crate) mod tests;
