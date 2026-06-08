use super::support::temp_dir;

#[test]
fn open_preview_supports_code_files() {
    let root = temp_dir("open_preview_supports_code_files");
    let source = root.join("src/main.rs");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, "fn main() {}\n").unwrap();

    let mut app = App::new(root.clone(), None);
    app.reveal_path(&source);
    app.open_preview();

    assert_eq!(
        app.preview.as_ref().map(|preview| preview.path.as_path()),
        Some(source.as_path())
    );
    assert!(app
        .preview
        .as_ref()
        .is_some_and(|preview| preview.preview.content.contains("fn main")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preview_focus_jk_scrolls_right_preview() {
    let root = temp_dir("preview_focus_jk_scrolls_right_preview");
    fs::create_dir_all(&root).unwrap();

    let mut app = App::new(root.clone(), None);
    app.focus_preview();
    app.next();
    assert_eq!(app.file_preview.scroll, 1);
    app.previous();
    assert_eq!(app.file_preview.scroll, 0);

    fs::remove_dir_all(root).unwrap();
}
