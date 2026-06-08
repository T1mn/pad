use super::support::temp_dir;

#[test]
fn display_options_toggle_numbers_and_zoom() {
    let root = temp_dir("display_options_toggle_numbers_and_zoom");
    fs::create_dir_all(&root).unwrap();

    let mut app = App::new(root.clone(), None);
    assert!(!app.show_line_numbers);
    app.toggle_line_numbers();
    assert!(app.show_line_numbers);
    app.zoom_text_in();
    app.zoom_text_in();
    app.zoom_text_in();
    assert_eq!(app.text_zoom, 2);
    app.zoom_text_out();
    app.zoom_text_out();
    app.zoom_text_out();
    app.zoom_text_out();
    assert_eq!(app.text_zoom, -1);

    fs::remove_dir_all(root).unwrap();
}
