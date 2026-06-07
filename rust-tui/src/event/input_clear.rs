use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn handle_shift_delete(app: &mut App, key: KeyEvent) -> bool {
    if key.code != KeyCode::Delete || !key.modifiers.contains(KeyModifiers::SHIFT) {
        return false;
    }

    clear_active_text_input(app)
}

fn clear_active_text_input(app: &mut App) -> bool {
    if app.relay_popup_editing {
        app.relay_popup_buffer.clear();
        app.dirty = true;
        return true;
    }

    if app.relay_editing {
        app.relay_edit_buffer.clear();
        app.dirty = true;
        return true;
    }

    if app.telegram_editing {
        app.telegram_edit_buffer.clear();
        app.dirty = true;
        return true;
    }

    if app.sidebar.thread_meta_editing {
        app.sidebar.thread_meta_buffer.clear();
        app.dirty = true;
        return true;
    }

    if app.mode == Mode::Settings && app.settings_searching {
        app.settings_search.clear();
        app.settings_selected = 0;
        app.dirty = true;
        return true;
    }

    match app.mode {
        Mode::Search => {
            app.search_query.clear();
            app.invalidate_sidebar_visible_cache();
            app.sync_sidebar_selection();
            app.invalidate_preview();
            app.dirty = true;
            true
        }
        Mode::TreeSearch => {
            let Some(tree) = app.sidebar.file_tree.as_mut() else {
                return false;
            };
            tree.clear_search_query();
            app.update_file_preview();
            app.dirty = true;
            true
        }
        Mode::FuzzyPicker => {
            let Some(picker) = app.fuzzy_picker.as_mut() else {
                return false;
            };
            picker.clear_query();
            app.dirty = true;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Mode;
    use crate::tree::FileTree;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn shift_delete() -> KeyEvent {
        KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT)
    }

    fn temp_tree_dir(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pad-input-clear-{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("alpha.txt"), "alpha").unwrap();
        fs::write(dir.join("beta.txt"), "beta").unwrap();
        dir
    }

    #[test]
    fn plain_delete_does_not_clear_search() {
        let mut app = App::new();
        app.mode = Mode::Search;
        app.search_query = "keep".into();

        assert!(!handle_shift_delete(
            &mut app,
            KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)
        ));
        assert_eq!(app.search_query, "keep");
    }

    #[test]
    fn shift_delete_clears_panel_search() {
        let mut app = App::new();
        app.mode = Mode::Search;
        app.search_query = "abc".into();

        assert!(handle_shift_delete(&mut app, shift_delete()));
        assert!(app.search_query.is_empty());
        assert!(app.dirty);
    }

    #[test]
    fn shift_delete_clears_settings_search_and_resets_selection() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_searching = true;
        app.settings_search = "relay".into();
        app.settings_selected = 3;

        assert!(handle_shift_delete(&mut app, shift_delete()));
        assert!(app.settings_search.is_empty());
        assert_eq!(app.settings_selected, 0);
    }

    #[test]
    fn shift_delete_clears_tree_search_without_leaving_search_mode() {
        let dir = temp_tree_dir("tree");
        let mut tree = FileTree::new(dir.clone());
        tree.start_search();
        tree.search_input('a');

        let mut app = App::new();
        app.mode = Mode::TreeSearch;
        app.sidebar.file_tree = Some(tree);

        assert!(handle_shift_delete(&mut app, shift_delete()));
        let tree = app.sidebar.file_tree.as_ref().unwrap();
        assert!(tree.search_query.is_empty());
        assert!(tree.mode == crate::tree::TreeMode::Search);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn shift_delete_clears_edit_buffers() {
        type EditCase = (fn(&mut App), fn(&App) -> &str);
        let cases: &[EditCase] = &[
            (
                |app| {
                    app.relay_popup_editing = true;
                    app.relay_popup_buffer = "popup".into();
                },
                |app| &app.relay_popup_buffer,
            ),
            (
                |app| {
                    app.relay_editing = true;
                    app.relay_edit_buffer = "relay".into();
                },
                |app| &app.relay_edit_buffer,
            ),
            (
                |app| {
                    app.telegram_editing = true;
                    app.telegram_edit_buffer = "telegram".into();
                },
                |app| &app.telegram_edit_buffer,
            ),
            (
                |app| {
                    app.sidebar.thread_meta_editing = true;
                    app.sidebar.thread_meta_buffer = "title".into();
                },
                |app| &app.sidebar.thread_meta_buffer,
            ),
        ];

        for (setup, buffer) in cases {
            let mut app = App::new();
            setup(&mut app);
            assert!(handle_shift_delete(&mut app, shift_delete()));
            assert!(buffer(&app).is_empty());
        }
    }
}
