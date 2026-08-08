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
