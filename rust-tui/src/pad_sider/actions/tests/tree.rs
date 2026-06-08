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
