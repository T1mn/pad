use super::support::temp_dir;

#[test]
fn idle_tick_without_content_changes_keeps_dirty_clear() {
    let root = temp_dir("idle_tick_without_content_changes_keeps_dirty_clear");
    fs::create_dir_all(&root).unwrap();

    let mut app = App::new(root.clone(), None);
    assert!(app.take_dirty());
    app.last_refresh = Instant::now() - Duration::from_secs(3);

    assert!(!app.tick());
    assert!(!app.dirty);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tick_refreshes_preview_without_rebuilding_tree() {
    let root = temp_dir("tick_refreshes_preview_without_rebuilding_tree");
    fs::create_dir_all(&root).unwrap();
    let selected = root.join("notes.md");
    let added_later = root.join("added.md");
    fs::write(&selected, "# before").unwrap();

    let mut app = App::new(root.clone(), None);
    app.reveal_path(&selected);
    assert!(app.file_preview.content.contains("before"));
    assert!(app.take_dirty());

    fs::write(&selected, "# after").unwrap();
    fs::write(&added_later, "# later").unwrap();
    app.last_refresh = Instant::now() - Duration::from_secs(3);

    assert!(app.tick());
    assert!(app.dirty);
    assert!(app.file_preview.content.contains("after"));
    assert!(!app.tree.iter().any(|row| row.path == added_later));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tick_eventually_rebuilds_tree_on_full_refresh_interval() {
    let root = temp_dir("tick_eventually_rebuilds_tree_on_full_refresh_interval");
    fs::create_dir_all(&root).unwrap();
    let added_later = root.join("added.md");

    let mut app = App::new(root.clone(), None);
    assert!(app.take_dirty());
    fs::write(&added_later, "# later").unwrap();
    app.last_refresh = Instant::now() - Duration::from_secs(3);
    app.last_full_refresh = Instant::now() - Duration::from_secs(31);

    assert!(app.tick());
    assert!(app.tree.iter().any(|row| row.path == added_later));

    fs::remove_dir_all(root).unwrap();
}
