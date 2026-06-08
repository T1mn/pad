use super::support::temp_dir;

#[test]
fn open_nearest_index_preview_uses_selected_directory_index() {
    let root = temp_dir("open_nearest_index_preview_uses_selected_directory_index");
    let docs = root.join("docs");
    fs::create_dir_all(&docs).unwrap();
    fs::write(docs.join("index.md"), "# docs").unwrap();
    fs::write(docs.join("guide.md"), "# guide").unwrap();

    let mut app = App::new(root.clone(), None);
    app.reveal_path(&docs.join("guide.md"));
    app.open_nearest_index_preview();

    assert_eq!(
        app.preview.as_ref().map(|preview| preview.path.as_path()),
        Some(docs.join("index.md").as_path())
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_index_preview_opens_from_index_map() {
    let root = temp_dir("selected_index_preview_opens_from_index_map");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("index.md"), "# root").unwrap();
    fs::write(root.join("docs/index.md"), "# docs").unwrap();

    let mut app = App::new(root.clone(), None);
    app.nav_mode = NavMode::IndexMap;
    app.focus = Focus::IndexMap;
    app.index_selected = 1;
    app.open_selected_index_preview();

    assert_eq!(
        app.preview.as_ref().map(|preview| preview.path.as_path()),
        Some(root.join("docs/index.md").as_path())
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reveal_selected_index_returns_to_tree() {
    let root = temp_dir("reveal_selected_index_returns_to_tree");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("index.md"), "# root").unwrap();
    fs::write(root.join("docs/index.md"), "# docs").unwrap();

    let mut app = App::new(root.clone(), None);
    app.nav_mode = NavMode::IndexMap;
    app.focus = Focus::IndexMap;
    app.index_selected = 1;
    app.reveal_selected_index_in_tree();

    assert_eq!(app.selected_path(), Some(&root.join("docs/index.md")));

    fs::remove_dir_all(root).unwrap();
}
