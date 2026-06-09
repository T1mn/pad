use super::{scan_directories, FuzzyPicker};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;

#[test]
fn shift_delete_clears_query() {
    let mut picker = FuzzyPicker::new(vec!["alpha".into(), "beta".into()]);

    picker.handle_input(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert_eq!(picker.query, "z");
    assert!(picker.filtered.is_empty());

    picker.handle_input(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
    assert!(picker.query.is_empty());
    assert_eq!(picker.filtered.len(), 2);
}

#[test]
fn scan_directories_skips_hidden_dirs_and_sorts() {
    let root = crate::test_support::temp_path("pad-fuzzy", "scan");
    fs::create_dir_all(root.join("zeta")).unwrap();
    fs::create_dir_all(root.join("alpha/nested")).unwrap();
    fs::create_dir_all(root.join(".hidden")).unwrap();

    let root_str = root.to_string_lossy();
    let dirs = scan_directories(&root_str, 2);

    assert_eq!(
        dirs,
        vec![
            root_str.to_string(),
            root.join("alpha").to_string_lossy().to_string(),
            root.join("alpha/nested").to_string_lossy().to_string(),
            root.join("zeta").to_string_lossy().to_string(),
        ]
    );
    let hidden = root.join(".hidden").to_string_lossy().to_string();
    assert!(!dirs.iter().any(|path| path == &hidden));

    fs::remove_dir_all(root).unwrap();
}
