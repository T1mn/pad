use super::super::app::{App, Focus, NavMode};

impl App {
    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Tree | Focus::IndexMap | Focus::CodexRuns => Focus::Preview,
            Focus::Preview => self.active_nav_focus(),
        };
    }

    pub fn focus_tree(&mut self) {
        self.set_tree_mode();
    }

    pub fn focus_preview(&mut self) {
        self.focus = Focus::Preview;
    }

    pub fn focus_codex_runs(&mut self) {
        self.set_codex_runs_mode();
    }

    pub fn focus_active_nav(&mut self) {
        self.focus = self.active_nav_focus();
    }

    pub fn active_nav_focus(&self) -> Focus {
        match self.nav_mode {
            NavMode::Tree => Focus::Tree,
            NavMode::IndexMap => Focus::IndexMap,
            NavMode::CodexRuns => Focus::CodexRuns,
        }
    }
}
