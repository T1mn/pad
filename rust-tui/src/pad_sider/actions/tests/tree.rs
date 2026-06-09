use super::support::temp_dir;

#[test]
fn reveal_path_expands_parents_and_selects_file() {
    let root = temp_dir("reveal_path_expands_parents_and_selects_file");
    let target = root.join("docs/guide/readme.md");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, "# guide").unwrap();

    let mut app = App::new(root.clone(), None);
    app.reveal_path(&target);

    assert_eq!(app.selected_path(), Some(&target));
    assert!(app.expanded.contains(&root.join("docs")));
    assert!(app.expanded.contains(&root.join("docs/guide")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn toggle_selected_collapses_directory_and_keeps_preview_on_directory() {
    let root = temp_dir("toggle_selected_collapses_directory_and_keeps_preview_on_directory");
    let docs = root.join("docs");
    let target = docs.join("guide.md");
    fs::create_dir_all(&docs).unwrap();
    fs::write(&target, "# guide").unwrap();

    let mut app = App::new(root.clone(), None);
    app.reveal_path(&docs);
    assert_eq!(app.selected_path(), Some(&docs));

    app.toggle_selected();
    assert!(app.expanded.contains(&docs));

    app.toggle_selected();

    assert_eq!(app.selected_path(), Some(&docs));
    assert!(!app.expanded.contains(&docs));
    assert_eq!(app.file_preview.title, "docs");
    assert!(app.file_preview.content.contains("Directory selected"));

    fs::remove_dir_all(root).unwrap();
}
