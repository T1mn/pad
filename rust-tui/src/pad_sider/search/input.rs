use super::{filter, FileSearch, SearchAction};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub(super) fn handle_key(search: &mut FileSearch, key: KeyEvent) -> SearchAction {
    if key.kind != KeyEventKind::Press {
        return SearchAction::None;
    }

    match key.code {
        KeyCode::Esc => SearchAction::Cancel,
        KeyCode::Enter => search
            .selected_path()
            .map(|path| SearchAction::Submit(path.to_path_buf()))
            .unwrap_or(SearchAction::Cancel),
        KeyCode::Up => {
            if search.selected > 0 {
                search.selected -= 1;
            }
            SearchAction::None
        }
        KeyCode::Down => {
            if search.selected + 1 < search.filtered.len() {
                search.selected += 1;
            }
            SearchAction::None
        }
        KeyCode::Delete if key.modifiers.contains(KeyModifiers::SHIFT) => {
            if !search.query.is_empty() {
                search.query.clear();
                filter::update_filter(search);
            }
            SearchAction::None
        }
        KeyCode::Backspace => {
            if search.query.pop().is_some() {
                filter::update_filter(search);
            }
            SearchAction::None
        }
        KeyCode::Char(c) => {
            search.query.push(c);
            filter::update_filter(search);
            SearchAction::None
        }
        _ => SearchAction::None,
    }
}
