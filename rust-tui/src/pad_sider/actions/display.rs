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
