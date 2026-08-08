use ratatui::style::{Color, Modifier};

use super::*;
use crate::terminal_runtime::{
    CursorShape, EngineId, PaneId, PaneMetadata, TerminalCursor, TerminalSize, TerminalSnapshot,
    TransportId,
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
