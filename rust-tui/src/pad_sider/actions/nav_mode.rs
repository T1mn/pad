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

    pub fn set_codex_runs_mode(&mut self) {
        self.nav_mode = NavMode::CodexRuns;
        self.focus = Focus::CodexRuns;
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
            NavMode::CodexRuns => {
                self.nav_mode = NavMode::Tree;
                self.focus = Focus::Tree;
            }
        }
        self.refresh_file_preview();
    }
}

#[cfg(test)]
#[path = "nav_mode_tests.rs"]
mod tests;
