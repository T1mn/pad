use super::FileSearch;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;

fn temp_search_dir() -> std::path::PathBuf {
    let dir = crate::test_support::temp_path("pad-sider-search", "files");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("alpha.rs"), "fn alpha() {}").unwrap();
    fs::write(dir.join("beta.rs"), "fn beta() {}").unwrap();
    dir
}

#[test]
fn shift_delete_clears_query() {
    let dir = temp_search_dir();
    let mut search = FileSearch::new(&dir);

    search.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert_eq!(search.query, "z");
    assert!(search.filtered.is_empty());

    search.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
    assert!(search.query.is_empty());
    assert_eq!(search.filtered.len(), 2);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn backspace_on_empty_query_keeps_existing_filter() {
    let dir = temp_search_dir();
    let mut search = FileSearch::new(&dir);
    search.selected = 1;
    let filtered = search.filtered.clone();

    search.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert!(search.query.is_empty());
    assert_eq!(search.selected, 1);
    assert_eq!(search.filtered, filtered);

    fs::remove_dir_all(dir).unwrap();
}
