use super::super::app::{App, Focus, NavMode};
use std::time::{Duration, Instant};

const DOUBLE_I_WINDOW: Duration = Duration::from_millis(600);

impl App {
    pub fn press_index_toggle_key(&mut self) {
        self.press_index_toggle_key_at(Instant::now());
    }

    pub(crate) fn press_index_toggle_key_at(&mut self, now: Instant) {
        let is_double = self
            .last_index_toggle_key
            .map(|last| now.duration_since(last) <= DOUBLE_I_WINDOW)
            .unwrap_or(false);
        if is_double {
            self.toggle_nav_mode();
            self.last_index_toggle_key = None;
        } else {
            self.last_index_toggle_key = Some(now);
        }
    }

    pub fn set_tree_mode(&mut self) {
        self.nav_mode = NavMode::Tree;
        self.focus = Focus::Tree;
        self.refresh_file_preview();
    }

    fn toggle_nav_mode(&mut self) {
        match self.nav_mode {
            NavMode::Tree => {
                self.nav_mode = NavMode::IndexMap;
                self.focus = Focus::IndexMap;
            }
            NavMode::IndexMap => {
                self.nav_mode = NavMode::Tree;
                self.focus = Focus::Tree;
            }
        }
        self.refresh_file_preview();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_i_toggles_between_tree_and_index_map() {
        let mut app = App::new(std::env::temp_dir(), None);
        let now = Instant::now();

        app.press_index_toggle_key_at(now);
        assert_eq!(app.nav_mode, NavMode::Tree);

        app.press_index_toggle_key_at(now + Duration::from_millis(200));
        assert_eq!(app.nav_mode, NavMode::IndexMap);
        assert_eq!(app.focus, Focus::IndexMap);

        app.press_index_toggle_key_at(now + Duration::from_millis(900));
        app.press_index_toggle_key_at(now + Duration::from_millis(1000));
        assert_eq!(app.nav_mode, NavMode::Tree);
        assert_eq!(app.focus, Focus::Tree);
    }
}
