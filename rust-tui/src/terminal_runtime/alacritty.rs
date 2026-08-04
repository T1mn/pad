use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use alacritty_terminal::event::{Event as AlacrittyEvent, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll as AlacrittyScroll};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, TermMode};
use alacritty_terminal::vte::ansi::{
    Color as AlacrittyColor, CursorShape as AlacrittyCursorShape, NamedColor, Processor,
};
use alacritty_terminal::Term;

use super::model::TerminalEngineEvent;
use super::{
    CursorShape, EngineFactory, EngineId, TerminalCell, TerminalColor, TerminalCursor,
    TerminalEngine, TerminalError, TerminalMode, TerminalScroll, TerminalSize, TerminalSnapshot,
    TerminalViewport, TextAttributes,
};

pub const ALACRITTY_ENGINE_ID: &str = "alacritty";

#[derive(Clone, Copy, Debug, Default)]
pub struct AlacrittyEngineFactory;

impl EngineFactory for AlacrittyEngineFactory {
    fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError> {
        Ok(Box::new(AlacrittyEngine::new(size)))
    }
}

pub struct AlacrittyEngine {
    id: EngineId,
    size: TerminalSize,
    parser: Processor,
    listener: AlacrittyEventListener,
    term: Term<AlacrittyEventListener>,
}

impl AlacrittyEngine {
    pub fn new(size: TerminalSize) -> Self {
        let size = normalize_size(size);
        let dimensions = DimensionsAdapter(size);
        let listener = AlacrittyEventListener::new(size);
        Self {
            id: EngineId::new(ALACRITTY_ENGINE_ID),
            size,
            parser: Processor::new(),
            listener: listener.clone(),
            term: Term::new(Config::default(), &dimensions, listener),
        }
    }
}

fn normalize_size(size: TerminalSize) -> TerminalSize {
    TerminalSize::new(size.columns, size.rows)
}

impl TerminalEngine for AlacrittyEngine {
    fn id(&self) -> &EngineId {
        &self.id
    }

    fn feed(&mut self, bytes: &[u8]) -> Result<(), TerminalError> {
        self.parser.advance(&mut self.term, bytes);
        Ok(())
    }

    fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
        let size = normalize_size(size);
        self.term.resize(DimensionsAdapter(size));
        self.size = size;
        Ok(())
    }

    fn scroll(&mut self, scroll: TerminalScroll) -> Result<(), TerminalError> {
        let scroll = match scroll {
            TerminalScroll::Lines(delta) => AlacrittyScroll::Delta(delta),
            TerminalScroll::PageUp => AlacrittyScroll::PageUp,
            TerminalScroll::PageDown => AlacrittyScroll::PageDown,
            TerminalScroll::Top => AlacrittyScroll::Top,
            TerminalScroll::Bottom => AlacrittyScroll::Bottom,
        };
        self.term.scroll_display(scroll);
        Ok(())
    }

    fn snapshot(&self) -> TerminalSnapshot {
        let renderable = self.term.renderable_content();
        let display_offset = renderable.display_offset as i32;
        let mut snapshot = TerminalSnapshot::blank(self.size);
        snapshot.viewport = TerminalViewport {
            display_offset: renderable.display_offset,
            history_size: self.term.grid().history_size(),
        };

        for indexed in renderable.display_iter {
            let row = indexed.point.line.0 + display_offset;
            let column = indexed.point.column.0;
            if row < 0
                || row >= i32::from(self.size.rows)
                || column >= usize::from(self.size.columns)
            {
                continue;
            }
            let index = row as usize * usize::from(self.size.columns) + column;
            let symbol =
                convert_symbol(indexed.cell.c, indexed.cell.zerowidth(), indexed.cell.flags);
            snapshot.cells[index] = TerminalCell {
                symbol,
                foreground: convert_color(indexed.cell.fg),
                background: convert_color(indexed.cell.bg),
                attributes: convert_attributes(indexed.cell.flags),
            };
        }

        let cursor_row = renderable.cursor.point.line.0 + display_offset;
        snapshot.cursor = convert_cursor(
            renderable.cursor.shape,
            renderable.cursor.point.column.0,
            cursor_row,
            self.size,
        );
        snapshot.mode = TerminalMode {
            alternate_screen: renderable.mode.contains(TermMode::ALT_SCREEN),
            bracketed_paste: renderable.mode.contains(TermMode::BRACKETED_PASTE),
            mouse_reporting: renderable.mode.intersects(
                TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION,
            ),
            sgr_mouse: renderable.mode.contains(TermMode::SGR_MOUSE),
            application_cursor: renderable.mode.contains(TermMode::APP_CURSOR),
        };
        snapshot
    }

    fn drain_events(&mut self) -> Vec<TerminalEngineEvent> {
        self.listener.drain()
    }
}

#[derive(Clone)]
struct AlacrittyEventListener(Rc<AlacrittyEventState>);

struct AlacrittyEventState {
    events: RefCell<Vec<TerminalEngineEvent>>,
}

impl AlacrittyEventListener {
    fn new(_size: TerminalSize) -> Self {
        Self(Rc::new(AlacrittyEventState {
            events: RefCell::new(Vec::new()),
        }))
    }

    fn drain(&self) -> Vec<TerminalEngineEvent> {
        mem::take(&mut *self.0.events.borrow_mut())
    }
}

impl EventListener for AlacrittyEventListener {
    fn send_event(&self, event: AlacrittyEvent) {
        let event = match event {
            AlacrittyEvent::PtyWrite(text) => TerminalEngineEvent::PtyWrite(text.into_bytes()),
            AlacrittyEvent::TextAreaSizeRequest(_) => TerminalEngineEvent::UnsupportedRequest(
                "text area pixel size is unavailable".to_string(),
            ),
            AlacrittyEvent::Title(title) => TerminalEngineEvent::Title(Some(title)),
            AlacrittyEvent::ResetTitle => TerminalEngineEvent::Title(None),
            AlacrittyEvent::Bell => TerminalEngineEvent::Bell,
            AlacrittyEvent::Exit | AlacrittyEvent::ChildExit(_) => TerminalEngineEvent::Exit,
            AlacrittyEvent::ClipboardStore(clipboard, _) => {
                TerminalEngineEvent::UnsupportedRequest(format!("clipboard store ({clipboard:?})"))
            }
            AlacrittyEvent::ClipboardLoad(clipboard, _) => {
                TerminalEngineEvent::UnsupportedRequest(format!("clipboard load ({clipboard:?})"))
            }
            AlacrittyEvent::ColorRequest(index, _) => {
                TerminalEngineEvent::UnsupportedRequest(format!("dynamic color {index}"))
            }
            // These are renderer invalidations already reflected by snapshot
            // pulls; they require no transport or host-side response.
            AlacrittyEvent::MouseCursorDirty
            | AlacrittyEvent::CursorBlinkingChange
            | AlacrittyEvent::Wakeup => return,
        };
        self.0.events.borrow_mut().push(event);
    }
}

fn convert_symbol(character: char, zerowidth: Option<&[char]>, flags: Flags) -> String {
    // Alacritty stores the continuation column of a wide glyph as a synthetic
    // space. Exporting that space would make textual snapshots read `界 x`
    // instead of `界x`, and a cell renderer could overwrite the second half
    // of the wide glyph. Keep the column in the snapshot, but give it no text.
    if flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
        return String::new();
    }

    let mut symbol = String::from(character);
    if let Some(zerowidth) = zerowidth {
        symbol.extend(zerowidth);
    }
    symbol
}

#[derive(Clone, Copy)]
struct DimensionsAdapter(TerminalSize);

impl Dimensions for DimensionsAdapter {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        usize::from(self.0.rows)
    }

    fn columns(&self) -> usize {
        usize::from(self.0.columns)
    }
}

fn convert_color(color: AlacrittyColor) -> TerminalColor {
    match color {
        AlacrittyColor::Named(NamedColor::Foreground) => TerminalColor::DefaultForeground,
        AlacrittyColor::Named(NamedColor::Background) => TerminalColor::DefaultBackground,
        AlacrittyColor::Named(color) => TerminalColor::Named(color as u16),
        AlacrittyColor::Indexed(index) => TerminalColor::Indexed(index),
        AlacrittyColor::Spec(rgb) => TerminalColor::Rgb(rgb.r, rgb.g, rgb.b),
    }
}

fn convert_attributes(flags: Flags) -> TextAttributes {
    TextAttributes {
        bold: flags.contains(Flags::BOLD),
        dim: flags.contains(Flags::DIM),
        italic: flags.contains(Flags::ITALIC),
        underline: flags.intersects(Flags::ALL_UNDERLINES),
        inverse: flags.contains(Flags::INVERSE),
        hidden: flags.contains(Flags::HIDDEN),
        strikeout: flags.contains(Flags::STRIKEOUT),
    }
}

fn convert_cursor(
    shape: AlacrittyCursorShape,
    column: usize,
    row: i32,
    size: TerminalSize,
) -> Option<TerminalCursor> {
    let shape = match shape {
        AlacrittyCursorShape::Block => CursorShape::Block,
        AlacrittyCursorShape::Underline => CursorShape::Underline,
        AlacrittyCursorShape::Beam => CursorShape::Beam,
        AlacrittyCursorShape::HollowBlock => CursorShape::HollowBlock,
        AlacrittyCursorShape::Hidden => return None,
    };
    if row < 0 || row >= i32::from(size.rows) || column >= usize::from(size.columns) {
        return None;
    }
    Some(TerminalCursor {
        column: column as u16,
        row: row as u16,
        shape,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_indexes_a_scrollback_viewport_from_zero() {
        let mut engine = AlacrittyEngine::new(TerminalSize::new(4, 3));
        engine.feed(b"0\r\n1\r\n2\r\n3\r\n4\r\n5").unwrap();
        engine.scroll(TerminalScroll::Lines(2)).unwrap();

        let snapshot = engine.snapshot();
        assert_eq!(snapshot.row_text(0).as_deref(), Some("1"));
        assert_eq!(snapshot.row_text(1).as_deref(), Some("2"));
        assert_eq!(snapshot.row_text(2).as_deref(), Some("3"));
        assert_eq!(snapshot.cell(0, 0).unwrap().symbol, "1");
        assert_eq!(snapshot.cell(0, 2).unwrap().symbol, "3");
        assert_eq!(snapshot.cursor, None);
        assert_eq!(snapshot.viewport.display_offset, 2);
        assert_eq!(snapshot.viewport.history_size, 3);
    }

    #[test]
    fn scrollback_clamps_and_stays_anchored_during_output() {
        let mut engine = AlacrittyEngine::new(TerminalSize::new(4, 3));
        engine.feed(b"0\r\n1\r\n2\r\n3\r\n4\r\n5").unwrap();

        engine.scroll(TerminalScroll::Top).unwrap();
        let top = engine.snapshot();
        assert_eq!(top.viewport.display_offset, top.viewport.history_size);
        assert_eq!(top.row_text(0).as_deref(), Some("0"));

        engine.scroll(TerminalScroll::Lines(-1)).unwrap();
        let anchored = engine.snapshot();
        assert_eq!(anchored.row_text(0).as_deref(), Some("1"));
        assert_eq!(anchored.viewport.display_offset, 2);

        engine.feed(b"\r\n6").unwrap();
        let after_output = engine.snapshot();
        assert_eq!(after_output.row_text(0).as_deref(), Some("1"));
        assert_eq!(after_output.viewport.display_offset, 3);
        assert_eq!(after_output.viewport.history_size, 4);

        engine.scroll(TerminalScroll::Bottom).unwrap();
        let bottom = engine.snapshot();
        assert_eq!(bottom.viewport.display_offset, 0);
        assert_eq!(bottom.row_text(2).as_deref(), Some("6"));
    }

    #[test]
    fn alternate_screen_has_no_scrollback_and_restores_primary_viewport() {
        let mut engine = AlacrittyEngine::new(TerminalSize::new(4, 3));
        engine.feed(b"0\r\n1\r\n2\r\n3\r\n4\r\n5").unwrap();
        engine.scroll(TerminalScroll::Lines(2)).unwrap();

        engine.feed(b"\x1b[?1049halt").unwrap();
        engine.scroll(TerminalScroll::Top).unwrap();
        let alternate = engine.snapshot();
        assert!(alternate.mode.alternate_screen);
        assert_eq!(alternate.viewport, TerminalViewport::default());

        engine.feed(b"\x1b[?1049l").unwrap();
        let primary = engine.snapshot();
        assert!(!primary.mode.alternate_screen);
        assert_eq!(primary.viewport.display_offset, 2);
        assert_eq!(primary.row_text(0).as_deref(), Some("1"));
    }

    #[test]
    fn listener_maps_exit_without_a_terminal_event_loop() {
        let listener = AlacrittyEventListener::new(TerminalSize::new(4, 3));
        listener.send_event(AlacrittyEvent::Exit);

        assert_eq!(listener.drain(), vec![TerminalEngineEvent::Exit]);
    }
}
