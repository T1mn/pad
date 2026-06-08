use super::{FileTree, TreeMode};
use crate::text_match::contains_ignore_case;

impl FileTree {
    /// Activate search mode
    pub fn start_search(&mut self) {
        self.mode = TreeMode::Search;
        self.search_query.clear();
    }

    /// Cancel search
    pub fn cancel_search(&mut self) {
        self.mode = TreeMode::Normal;
        self.search_query.clear();
        self.refresh_entries(); // Show all entries again
    }

    /// Add character to search query
    pub fn search_input(&mut self, c: char) {
        if self.mode == TreeMode::Search {
            self.search_query.push(c);
            self.filter_entries();
        }
    }

    /// Remove last character from search query
    pub fn search_backspace(&mut self) {
        if self.mode == TreeMode::Search {
            self.search_query.pop();
            if self.search_query.is_empty() {
                self.refresh_entries();
            } else {
                self.filter_entries();
            }
        }
    }

    /// Clear the search query while staying in search mode
    pub fn clear_search_query(&mut self) {
        if self.mode == TreeMode::Search {
            self.search_query.clear();
            self.refresh_entries();
        }
    }

    /// Filter entries based on search query
    fn filter_entries(&mut self) {
        let all_entries = self.scan_directory(&self.current_path);

        self.entries = all_entries
            .into_iter()
            .filter(|e| {
                // Always keep ".."
                if e.name == ".." {
                    return true;
                }
                contains_ignore_case(&e.name, &self.search_query)
            })
            .collect();

        // Reset selection
        self.state.select(Some(0));
    }
}
