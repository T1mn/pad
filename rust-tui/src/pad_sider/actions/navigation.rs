use super::super::app::{App, Focus};

impl App {
    pub fn next(&mut self) {
        match self.focus {
            Focus::Tree => {
                if self.selected + 1 < self.tree.len() {
                    self.selected += 1;
                    self.refresh_selected();
                    self.refresh_file_preview();
                }
            }
            Focus::IndexMap => {
                if self.index_selected + 1 < self.index_rows.len() {
                    self.index_selected += 1;
                    self.refresh_file_preview();
                }
            }
            Focus::CodexRuns => {
                if self.codex_diff_selected + 1 < self.codex_diffs.len() {
                    self.codex_diff_selected += 1;
                    self.refresh_file_preview();
                }
            }
            Focus::Preview => self.file_preview_scroll_down(1),
        }
    }

    pub fn previous(&mut self) {
        match self.focus {
            Focus::Tree => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.refresh_selected();
                    self.refresh_file_preview();
                }
            }
            Focus::IndexMap => {
                if self.index_selected > 0 {
                    self.index_selected -= 1;
                    self.refresh_file_preview();
                }
            }
            Focus::CodexRuns => {
                if self.codex_diff_selected > 0 {
                    self.codex_diff_selected -= 1;
                    self.refresh_file_preview();
                }
            }
            Focus::Preview => self.file_preview_scroll_up(1),
        }
    }

    pub fn reset_position(&mut self) {
        match self.focus {
            Focus::Tree => {
                self.selected = 0;
                self.refresh_selected();
                self.refresh_file_preview();
            }
            Focus::IndexMap => {
                self.index_selected = 0;
                self.refresh_file_preview();
            }
            Focus::CodexRuns => {
                self.codex_diff_selected = 0;
                self.refresh_file_preview();
            }
            Focus::Preview => self.file_preview.scroll = 0,
        }
    }

    pub fn jump_bottom(&mut self) {
        match self.focus {
            Focus::Tree => {
                self.selected = self.tree.len().saturating_sub(1);
                self.refresh_selected();
                self.refresh_file_preview();
            }
            Focus::IndexMap => {
                self.index_selected = self.index_rows.len().saturating_sub(1);
                self.refresh_file_preview();
            }
            Focus::CodexRuns => {
                self.codex_diff_selected = self.codex_diffs.len().saturating_sub(1);
                self.refresh_file_preview();
            }
            Focus::Preview => self.file_preview.scroll = u16::MAX,
        }
    }
}
