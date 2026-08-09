use crossterm::event::KeyModifiers;

use super::*;

pub(crate) fn c_keeps_opening_the_global_index_in_native_mode() {
    crate::test_support::with_temp_home("pad-global-index", "native-c", |_home| {
        let mut app = App::new();

        assert!(handle_primary_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)
        ));
        assert!(matches!(app.mode, Mode::FuzzyPicker));
        assert!(app.fuzzy_picker.is_some());
    });
}
