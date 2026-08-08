mod display {
    use super::super::app::App;
    use super::super::search::FileSearch;

    impl App {
        pub fn open_search(&mut self) {
            self.search = Some(FileSearch::new(&self.cwd));
        }

        pub fn close_search(&mut self) {
            self.search = None;
        }

        pub fn toggle_help(&mut self) {
            self.show_help = !self.show_help;
        }

        pub fn close_help(&mut self) {
            self.show_help = false;
        }

        pub fn toggle_line_numbers(&mut self) {
            self.show_line_numbers = !self.show_line_numbers;
        }

        pub fn zoom_text_in(&mut self) {
            self.text_zoom = (self.text_zoom + 1).min(2);
        }

        pub fn zoom_text_out(&mut self) {
            self.text_zoom = (self.text_zoom - 1).max(-1);
        }
    }
}
mod focus {
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
}
mod index_map {
    use super::super::app::App;
    use std::path::PathBuf;

    impl App {
        pub fn open_nearest_index_preview(&mut self) {
            let Some(path) = self.nearest_index_path() else {
                return;
            };
            self.reveal_path(&path);
            self.open_preview_path(path);
        }

        pub fn open_selected_index_preview(&mut self) {
            let Some(path) = self.selected_index_path().cloned() else {
                return;
            };
            self.open_preview_path(path);
        }

        pub fn reveal_selected_index_in_tree(&mut self) {
            let Some(path) = self.selected_index_path().cloned() else {
                return;
            };
            self.reveal_path(&path);
            self.set_tree_mode();
        }

        fn nearest_index_path(&self) -> Option<PathBuf> {
            let selected = self.selected_path()?;
            let mut cursor = if selected.is_dir() {
                selected.as_path()
            } else {
                selected.parent()?
            };

            loop {
                if !cursor.starts_with(&self.cwd) {
                    return None;
                }
                let candidate = cursor.join("index.md");
                if candidate.is_file() {
                    return Some(candidate);
                }
                if cursor == self.cwd {
                    return None;
                }
                cursor = cursor.parent()?;
            }
        }
    }
}
mod nav_mode;
mod navigation;
mod preview {
    use super::super::app::App;
    use super::super::preview::FullscreenPreview;
    use std::path::PathBuf;

    impl App {
        pub fn open_preview(&mut self) {
            let Some(path) = self.selected_path().cloned() else {
                return;
            };
            self.open_preview_path(path);
        }

        pub(crate) fn open_preview_path(&mut self, path: PathBuf) {
            if !path.is_file() {
                return;
            }
            let preview = self.file_preview_cache.preview_for(&self.cwd, &path);
            self.preview = Some(FullscreenPreview { path, preview });
        }

        pub fn close_preview(&mut self) {
            self.preview = None;
        }

        pub fn preview_down(&mut self) {
            if let Some(preview) = self.preview.as_mut() {
                preview.preview.scroll = preview.preview.scroll.saturating_add(1);
            }
        }

        pub fn preview_up(&mut self) {
            if let Some(preview) = self.preview.as_mut() {
                preview.preview.scroll = preview.preview.scroll.saturating_sub(1);
            }
        }

        pub fn reset_preview(&mut self) {
            if let Some(preview) = self.preview.as_mut() {
                preview.preview.scroll = 0;
            }
        }

        pub fn preview_bottom(&mut self) {
            if let Some(preview) = self.preview.as_mut() {
                preview.preview.scroll = u16::MAX;
            }
        }

        pub fn file_preview_down(&mut self) {
            self.file_preview_scroll_down(8);
        }

        pub fn file_preview_up(&mut self) {
            self.file_preview_scroll_up(8);
        }

        pub fn file_preview_scroll_down(&mut self, amount: u16) {
            self.file_preview.scroll = self.file_preview.scroll.saturating_add(amount);
        }

        pub fn file_preview_scroll_up(&mut self, amount: u16) {
            self.file_preview.scroll = self.file_preview.scroll.saturating_sub(amount);
        }
    }
}
mod tree {
    use super::super::app::App;
    use std::path::Path;

    impl App {
        pub fn toggle_selected(&mut self) {
            let Some(row) = self.tree.get(self.selected) else {
                return;
            };
            if !row.is_dir || row.path == self.cwd {
                return;
            }
            let path = row.path.clone();
            if !self.expanded.insert(path.clone()) {
                self.expanded.remove(&path);
            }
            self.refresh();
        }

        pub fn reveal_path(&mut self, path: &Path) {
            if !path.starts_with(&self.cwd) {
                return;
            }

            self.expanded.insert(self.cwd.clone());
            let mut cursor = path.parent();
            while let Some(dir) = cursor {
                if !dir.starts_with(&self.cwd) {
                    break;
                }
                self.expanded.insert(dir.to_path_buf());
                if dir == self.cwd {
                    break;
                }
                cursor = dir.parent();
            }

            self.refresh();
            self.set_selected_path(path);
            self.refresh_selected();
            self.refresh_file_preview();
        }
    }
}

#[cfg(test)]
mod tests;
