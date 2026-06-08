use super::FuzzyPicker;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
