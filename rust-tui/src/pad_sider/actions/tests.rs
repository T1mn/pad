use super::super::app::{Focus, NavMode};
use super::App;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("pad_sider_{name}_{unique}"))
}
