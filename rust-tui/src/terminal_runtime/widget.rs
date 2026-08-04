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
mod tests {
    use ratatui::style::{Color, Modifier};

    use super::*;
    use crate::terminal_runtime::{
        CursorShape, EngineId, PaneId, PaneMetadata, TerminalCursor, TerminalSize,
        TerminalSnapshot, TransportId,
    };

    #[test]
    fn widget_renders_label_terminal_cells_and_cursor() {
        let mut terminal = TerminalSnapshot::blank(TerminalSize::new(4, 2));
        terminal.cells[0].symbol = "O".to_string();
        terminal.cells[0].foreground = TerminalColor::Rgb(12, 34, 56);
        terminal.cells[0].attributes.bold = true;
        terminal.cursor = Some(TerminalCursor {
            column: 1,
            row: 0,
            shape: CursorShape::Block,
        });
        let frame = PaneFrame {
            metadata: PaneMetadata {
                id: PaneId::new("codex-1"),
                label: "Codex".to_string(),
                engine_id: EngineId::new("alacritty"),
                transport_id: TransportId::new("replay"),
            },
            terminal,
        };
        let area = Rect::new(0, 0, 10, 4);
        let mut buffer = Buffer::empty(area);

        TerminalPaneWidget::new(&frame)
            .focused(true)
            .render(area, &mut buffer);

        assert_eq!(buffer[(1, 1)].symbol(), "O");
        assert_eq!(buffer[(1, 1)].fg, Color::Rgb(12, 34, 56));
        assert!(buffer[(1, 1)].modifier.contains(Modifier::BOLD));
        assert!(buffer[(2, 1)].modifier.contains(Modifier::REVERSED));
        let border: String = (0..area.width)
            .map(|column| buffer[(column, 0)].symbol())
            .collect();
        assert!(border.contains("Codex"));
    }

    #[test]
    fn widget_clips_snapshot_to_inner_area() {
        let terminal = TerminalSnapshot::blank(TerminalSize::new(80, 24));
        let frame = PaneFrame {
            metadata: PaneMetadata {
                id: PaneId::new("pane"),
                label: "A label longer than the pane".to_string(),
                engine_id: EngineId::new("alacritty"),
                transport_id: TransportId::new("replay"),
            },
            terminal,
        };
        let area = Rect::new(5, 7, 3, 2);
        let mut buffer = Buffer::empty(area);

        TerminalPaneWidget::new(&frame).render(area, &mut buffer);

        assert_eq!(buffer.area, area);
    }

    #[test]
    fn widget_does_not_render_wide_grapheme_across_right_border() {
        let mut terminal = TerminalSnapshot::blank(TerminalSize::new(2, 1));
        terminal.cells[1].symbol = "界".to_string();
        let frame = PaneFrame {
            metadata: PaneMetadata {
                id: PaneId::new("wide-edge"),
                label: "W".to_string(),
                engine_id: EngineId::new("alacritty"),
                transport_id: TransportId::new("replay"),
            },
            terminal,
        };
        let area = Rect::new(0, 0, 4, 3);
        let mut buffer = Buffer::empty(area);

        TerminalPaneWidget::new(&frame).render(area, &mut buffer);

        assert_eq!(buffer[(2, 1)].symbol(), " ");
        assert_eq!(buffer[(3, 1)].symbol(), "│");
    }

    #[test]
    fn underline_cursor_uses_underline_fallback() {
        assert_eq!(
            cursor_modifier(CursorShape::Underline),
            Modifier::UNDERLINED
        );
        assert_eq!(cursor_modifier(CursorShape::Beam), Modifier::REVERSED);
    }

    #[test]
    fn cursor_modifier_preserves_the_cells_existing_style() {
        let mut terminal = TerminalSnapshot::blank(TerminalSize::new(1, 1));
        terminal.cells[0].foreground = TerminalColor::Rgb(10, 20, 30);
        terminal.cells[0].background = TerminalColor::Rgb(40, 50, 60);
        terminal.cells[0].attributes.bold = true;
        terminal.cursor = Some(TerminalCursor {
            column: 0,
            row: 0,
            shape: CursorShape::Block,
        });
        let frame = PaneFrame {
            metadata: PaneMetadata {
                id: PaneId::new("styled-cursor"),
                label: "Styled".to_string(),
                engine_id: EngineId::new("alacritty"),
                transport_id: TransportId::new("replay"),
            },
            terminal,
        };
        let area = Rect::new(0, 0, 3, 3);
        let mut buffer = Buffer::empty(area);

        TerminalPaneWidget::new(&frame)
            .focused(true)
            .render(area, &mut buffer);

        let cursor = &buffer[(1, 1)];
        assert_eq!(cursor.fg, Color::Rgb(10, 20, 30));
        assert_eq!(cursor.bg, Color::Rgb(40, 50, 60));
        assert!(cursor.modifier.contains(Modifier::BOLD));
        assert!(cursor.modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn terminal_attributes_map_to_ratatui_modifiers() {
        let modifiers = attribute_modifiers(TextAttributes {
            bold: true,
            dim: true,
            italic: true,
            underline: true,
            inverse: true,
            hidden: true,
            strikeout: true,
        });
        assert!(modifiers.contains(Modifier::BOLD));
        assert!(modifiers.contains(Modifier::DIM));
        assert!(modifiers.contains(Modifier::ITALIC));
        assert!(modifiers.contains(Modifier::UNDERLINED));
        assert!(modifiers.contains(Modifier::REVERSED));
        assert!(modifiers.contains(Modifier::HIDDEN));
        assert!(modifiers.contains(Modifier::CROSSED_OUT));
    }
}
