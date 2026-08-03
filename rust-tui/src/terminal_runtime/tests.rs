use std::sync::{Arc, Mutex};
use std::thread;

use super::*;

#[test]
fn alacritty_engine_parses_ansi_and_resize() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(12, 3));
    engine.feed(b"hello\r\n\x1b[31mred\x1b[0m").unwrap();

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.row_text(0).as_deref(), Some("hello"));
    assert_eq!(snapshot.row_text(1).as_deref(), Some("red"));
    assert_eq!(
        snapshot.cell(0, 1).map(|cell| cell.foreground),
        Some(TerminalColor::Named(1))
    );

    engine.resize(TerminalSize::new(20, 4)).unwrap();
    let resized = engine.snapshot();
    assert_eq!(resized.size, TerminalSize::new(20, 4));
    assert_eq!(resized.cells.len(), 80);
}

#[test]
fn alacritty_engine_tracks_tui_modes_and_unicode() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(12, 3));
    engine.feed("界\x1b[?1049h\x1b[?2004h".as_bytes()).unwrap();

    let snapshot = engine.snapshot();
    assert!(snapshot.mode.alternate_screen);
    assert!(snapshot.mode.bracketed_paste);

    engine.feed(b"\x1b[?2004l\x1b[?1049l").unwrap();
    let restored = engine.snapshot();
    assert!(!restored.mode.alternate_screen);
    assert!(!restored.mode.bracketed_paste);
    assert_eq!(
        restored.cell(0, 0).map(|cell| cell.symbol.as_str()),
        Some("界")
    );
}

#[test]
fn alacritty_engine_emits_dsr_and_device_attribute_replies() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(12, 4));
    engine.feed(b"\x1b[2;3H\x1b[5n\x1b[6n\x1b[c").unwrap();

    assert_eq!(
        engine.drain_events(),
        vec![
            TerminalEngineEvent::PtyWrite(b"\x1b[0n".to_vec()),
            TerminalEngineEvent::PtyWrite(b"\x1b[2;3R".to_vec()),
            TerminalEngineEvent::PtyWrite(b"\x1b[?6c".to_vec()),
        ]
    );
    assert!(engine.drain_events().is_empty());

    engine.feed(b"\x1b[>c").unwrap();
    let secondary = engine.drain_events();
    assert_eq!(secondary.len(), 1);
    let TerminalEngineEvent::PtyWrite(reply) = &secondary[0] else {
        panic!("secondary device attributes must write to the PTY");
    };
    assert!(reply.starts_with(b"\x1b[>0;"), "reply={reply:?}");
    assert!(reply.ends_with(b";1c"), "reply={reply:?}");
}

#[test]
fn alacritty_engine_reports_character_size_without_fabricating_pixel_size() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(80, 24));
    engine.feed(b"\x1b[18t\x1b[14t").unwrap();
    assert_eq!(
        engine.drain_events(),
        vec![
            TerminalEngineEvent::PtyWrite(b"\x1b[8;24;80t".to_vec()),
            TerminalEngineEvent::UnsupportedRequest(
                "text area pixel size is unavailable".to_string(),
            ),
        ]
    );

    engine.resize(TerminalSize::new(100, 30)).unwrap();
    engine.feed(b"\x1b[18t\x1b[14t").unwrap();
    assert_eq!(
        engine.drain_events(),
        vec![
            TerminalEngineEvent::PtyWrite(b"\x1b[8;30;100t".to_vec()),
            TerminalEngineEvent::UnsupportedRequest(
                "text area pixel size is unavailable".to_string(),
            ),
        ]
    );
}

#[test]
fn alacritty_engine_emits_title_reset_bell_and_explicit_unsupported_requests() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(12, 4));
    engine
        .feed(b"\x1b[22t\x1b]0;build log\x07\x07\x1b[23t\x1b]10;?\x1b\\")
        .unwrap();

    let events = engine.drain_events();
    assert_eq!(
        events[0],
        TerminalEngineEvent::Title(Some("build log".into()))
    );
    assert_eq!(events[1], TerminalEngineEvent::Bell);
    assert_eq!(events[2], TerminalEngineEvent::Title(None));
    assert!(matches!(
        events.get(3),
        Some(TerminalEngineEvent::UnsupportedRequest(request))
            if request.starts_with("dynamic color ")
    ));
    assert_eq!(events.len(), 4);
}

#[test]
fn alacritty_engine_preserves_wide_and_combining_cells() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(10, 2));
    let text = "界́x";

    // Split both the wide glyph and combining mark across feeds to exercise
    // the parser's streaming UTF-8 state.
    for byte in text.as_bytes() {
        engine.feed(std::slice::from_ref(byte)).unwrap();
    }

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.row_text(0).as_deref(), Some(text));
    assert_eq!(snapshot.cell(0, 0).unwrap().symbol, "界́");
    assert_eq!(snapshot.cell(1, 0).unwrap().symbol, "");
    assert_eq!(snapshot.cell(2, 0).unwrap().symbol, "x");
    assert_eq!(
        snapshot.cursor,
        Some(TerminalCursor {
            column: 3,
            row: 0,
            shape: CursorShape::Block,
        })
    );
}

#[test]
fn alacritty_engine_keeps_wide_wrap_placeholders_textless() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(4, 2));
    engine.feed("abc界".as_bytes()).unwrap();

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.row_text(0).as_deref(), Some("abc"));
    assert_eq!(snapshot.row_text(1).as_deref(), Some("界"));
    assert_eq!(snapshot.cell(0, 0).unwrap().symbol, "a");
    assert_eq!(snapshot.cell(1, 0).unwrap().symbol, "b");
    assert_eq!(snapshot.cell(2, 0).unwrap().symbol, "c");
    assert_eq!(snapshot.cell(3, 0).unwrap().symbol, "");
    assert_eq!(snapshot.cell(0, 1).unwrap().symbol, "界");
    assert_eq!(snapshot.cell(1, 1).unwrap().symbol, "");
}

#[test]
fn terminal_snapshot_row_text_only_trims_empty_ascii_cells() {
    let mut snapshot = TerminalSnapshot::blank(TerminalSize::new(4, 1));
    snapshot.cells[0].symbol = "a".to_string();
    snapshot.cells[1].symbol = "\u{a0}".to_string();

    assert_eq!(snapshot.row_text(0).as_deref(), Some("a\u{a0}"));
}

#[test]
fn alacritty_engine_preserves_ansi_colors_and_attributes() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(8, 2));
    engine
        .feed(b"\x1b[1;2;3;4;7;8;9;38;5;202;48;2;1;2;3mX\x1b[0mY")
        .unwrap();

    let snapshot = engine.snapshot();
    let styled = snapshot.cell(0, 0).unwrap();
    assert_eq!(styled.foreground, TerminalColor::Indexed(202));
    assert_eq!(styled.background, TerminalColor::Rgb(1, 2, 3));
    assert_eq!(
        styled.attributes,
        TextAttributes {
            bold: true,
            dim: true,
            italic: true,
            underline: true,
            inverse: true,
            hidden: true,
            strikeout: true,
        }
    );

    let reset = snapshot.cell(1, 0).unwrap();
    assert_eq!(reset.foreground, TerminalColor::DefaultForeground);
    assert_eq!(reset.background, TerminalColor::DefaultBackground);
    assert_eq!(reset.attributes, TextAttributes::default());
}

#[test]
fn alacritty_engine_tracks_cursor_shape_visibility_position_and_modes() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(8, 4));
    engine
        .feed(b"\x1b[3;5H\x1b[5 q\x1b[?1h\x1b[?1002h")
        .unwrap();

    let snapshot = engine.snapshot();
    assert_eq!(
        snapshot.cursor,
        Some(TerminalCursor {
            column: 4,
            row: 2,
            shape: CursorShape::Beam,
        })
    );
    assert!(snapshot.mode.application_cursor);
    assert!(snapshot.mode.mouse_reporting);

    engine.feed(b"\x1b[?25l\x1b[?1l\x1b[?1002l").unwrap();
    let hidden = engine.snapshot();
    assert_eq!(hidden.cursor, None);
    assert!(!hidden.mode.application_cursor);
    assert!(!hidden.mode.mouse_reporting);

    engine.feed(b"\x1b[?25h\x1b[4 q").unwrap();
    assert_eq!(
        engine.snapshot().cursor.map(|cursor| cursor.shape),
        Some(CursorShape::Underline)
    );
}

#[test]
fn alacritty_engine_restores_primary_screen_and_cursor_after_alt_screen() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(8, 3));
    engine.feed(b"main\x1b[2;3H\x1b[?1049halt").unwrap();

    let alternate = engine.snapshot();
    assert!(alternate.mode.alternate_screen);
    // Alacritty initializes the alternate buffer cursor from the primary
    // cursor, so output starts at the saved position on the cleared screen.
    assert_eq!(alternate.row_text(0).as_deref(), Some(""));
    assert_eq!(alternate.row_text(1).as_deref(), Some("  alt"));

    engine.feed(b"\x1b[?1049l").unwrap();
    let restored = engine.snapshot();
    assert!(!restored.mode.alternate_screen);
    assert_eq!(restored.row_text(0).as_deref(), Some("main"));
    assert_eq!(
        restored.cursor,
        Some(TerminalCursor {
            column: 2,
            row: 1,
            shape: CursorShape::Block,
        })
    );
}

#[test]
fn alacritty_engine_snapshots_scrolled_output_in_row_major_order() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(5, 3));
    engine.feed(b"0\r\n1\r\n2\r\n3\r\n4").unwrap();

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.cells.len(), 15);
    assert_eq!(snapshot.row_text(0).as_deref(), Some("2"));
    assert_eq!(snapshot.row_text(1).as_deref(), Some("3"));
    assert_eq!(snapshot.row_text(2).as_deref(), Some("4"));
    assert_eq!(snapshot.cell(0, 0).unwrap().symbol, "2");
    assert_eq!(snapshot.cell(0, 1).unwrap().symbol, "3");
    assert_eq!(snapshot.cell(0, 2).unwrap().symbol, "4");
    assert!(snapshot.cell(5, 0).is_none());
    assert!(snapshot.cell(0, 3).is_none());
}

#[test]
fn alacritty_engine_reflows_content_across_resize() {
    let mut engine = AlacrittyEngine::new(TerminalSize::new(6, 3));
    engine.feed(b"abcdefghi").unwrap();

    engine.resize(TerminalSize::new(4, 3)).unwrap();
    let narrowed = engine.snapshot();
    // Reflow anchors the cursor and moves the oldest wrapped segment into
    // scrollback; it must not discard it, since widening pulls it back in.
    assert_eq!(narrowed.row_text(0).as_deref(), Some("efgh"));
    assert_eq!(narrowed.row_text(1).as_deref(), Some("i"));
    assert_eq!(narrowed.row_text(2).as_deref(), Some(""));

    engine.resize(TerminalSize::new(8, 3)).unwrap();
    let widened = engine.snapshot();
    assert_eq!(widened.row_text(0).as_deref(), Some("abcdefgh"));
    assert_eq!(widened.row_text(1).as_deref(), Some("i"));
}

#[test]
fn alacritty_engine_normalizes_zero_dimensions_at_boundaries() {
    let invalid = TerminalSize {
        columns: 0,
        rows: 0,
    };
    let mut engine = AlacrittyEngine::new(invalid);
    assert_eq!(engine.snapshot().size, TerminalSize::new(1, 1));

    engine.resize(invalid).unwrap();
    assert_eq!(engine.snapshot().size, TerminalSize::new(1, 1));
}

#[test]
fn worker_runtime_supports_multiple_registered_engines() {
    let created_on = Arc::new(Mutex::new(Vec::new()));
    let mut registry = EngineRegistry::default();
    registry.register(
        EngineId::new("probe"),
        ProbeFactory {
            created_on: created_on.clone(),
        },
    );
    registry.register(
        EngineId::new(ALACRITTY_ENGINE_ID),
        RecordingAlacrittyFactory {
            created_on: created_on.clone(),
        },
    );

    let runtime = EngineRuntime::start(2, registry).unwrap();
    let first = PaneId::new("probe-pane");
    let second = pane_on_other_shard(&runtime, &first);
    runtime
        .open(
            first.clone(),
            EngineId::new("probe"),
            TerminalSize::new(10, 2),
        )
        .unwrap();
    runtime
        .open(
            second.clone(),
            EngineId::new(ALACRITTY_ENGINE_ID),
            TerminalSize::new(10, 2),
        )
        .unwrap();

    runtime.feed(&first, b"one".to_vec()).unwrap();
    runtime.feed(&second, b"two".to_vec()).unwrap();
    assert_eq!(
        runtime.snapshot(&first).unwrap().row_text(0).as_deref(),
        Some("one")
    );
    assert_eq!(
        runtime.snapshot(&second).unwrap().row_text(0).as_deref(),
        Some("two")
    );

    let main_thread = thread::current().id();
    let worker_threads = created_on.lock().unwrap();
    assert_eq!(worker_threads.len(), 2);
    assert!(worker_threads
        .iter()
        .all(|thread_id| *thread_id != main_thread));
    assert_ne!(worker_threads[0], worker_threads[1]);
}

#[test]
fn worker_runtime_drains_engine_events_in_parser_order() {
    let mut registry = EngineRegistry::default();
    registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
    let runtime = EngineRuntime::start(1, registry).unwrap();
    let pane_id = PaneId::new("event-pane");
    runtime
        .open(
            pane_id.clone(),
            EngineId::new(ALACRITTY_ENGINE_ID),
            TerminalSize::new(20, 5),
        )
        .unwrap();

    runtime
        .feed(&pane_id, b"\x1b[5n\x1b]0;worker\x07\x07".to_vec())
        .unwrap();
    assert_eq!(
        runtime.drain_events(&pane_id).unwrap(),
        vec![
            TerminalEngineEvent::PtyWrite(b"\x1b[0n".to_vec()),
            TerminalEngineEvent::Title(Some("worker".into())),
            TerminalEngineEvent::Bell,
        ]
    );
    assert!(runtime.drain_events(&pane_id).unwrap().is_empty());

    runtime.close(&pane_id).unwrap();
    assert_eq!(
        runtime.drain_events(&pane_id).unwrap_err().to_string(),
        "terminal pane 'event-pane' is not open"
    );
}

#[test]
fn pane_runtime_keeps_label_outside_terminal_engine() {
    let mut registry = EngineRegistry::default();
    registry.register(EngineId::new(ALACRITTY_ENGINE_ID), AlacrittyEngineFactory);
    let engines = EngineRuntime::start(1, registry).unwrap();
    let mut panes = PaneRuntime::new(engines);
    let pane_id = PaneId::new("codex-1");
    panes
        .open(
            PaneSpec {
                id: pane_id.clone(),
                label: "Codex · API".to_string(),
                engine_id: EngineId::new(ALACRITTY_ENGINE_ID),
                transport_id: TransportId::new("tmux-control"),
            },
            TerminalSize::new(20, 3),
        )
        .unwrap();
    panes.feed_output(&pane_id, b"ready".to_vec()).unwrap();

    let frame = panes.frame(&pane_id).unwrap();
    assert_eq!(frame.metadata.label, "Codex · API");
    assert_eq!(frame.terminal.row_text(0).as_deref(), Some("ready"));
}

struct RecordingAlacrittyFactory {
    created_on: Arc<Mutex<Vec<thread::ThreadId>>>,
}

impl EngineFactory for RecordingAlacrittyFactory {
    fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError> {
        self.created_on.lock().unwrap().push(thread::current().id());
        Ok(Box::new(AlacrittyEngine::new(size)))
    }
}

struct ProbeFactory {
    created_on: Arc<Mutex<Vec<thread::ThreadId>>>,
}

impl EngineFactory for ProbeFactory {
    fn create(&self, size: TerminalSize) -> Result<Box<dyn TerminalEngine>, TerminalError> {
        self.created_on.lock().unwrap().push(thread::current().id());
        Ok(Box::new(ProbeEngine {
            id: EngineId::new("probe"),
            snapshot: TerminalSnapshot::blank(size),
        }))
    }
}

struct ProbeEngine {
    id: EngineId,
    snapshot: TerminalSnapshot,
}

impl TerminalEngine for ProbeEngine {
    fn id(&self) -> &EngineId {
        &self.id
    }

    fn feed(&mut self, bytes: &[u8]) -> Result<(), TerminalError> {
        let text = String::from_utf8(bytes.to_vec())
            .map_err(|error| TerminalError::new(error.to_string()))?;
        for (cell, character) in self.snapshot.cells.iter_mut().zip(text.chars()) {
            cell.symbol = character.to_string();
        }
        Ok(())
    }

    fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
        self.snapshot = TerminalSnapshot::blank(size);
        Ok(())
    }

    fn snapshot(&self) -> TerminalSnapshot {
        self.snapshot.clone()
    }
}

fn pane_on_other_shard(runtime: &EngineRuntime, first: &PaneId) -> PaneId {
    let first_shard = runtime.shard_index(first);
    (0..100)
        .map(|index| PaneId::new(format!("probe-pane-{index}")))
        .find(|pane_id| runtime.shard_index(pane_id) != first_shard)
        .expect("two worker shards should accept different pane hashes")
}
