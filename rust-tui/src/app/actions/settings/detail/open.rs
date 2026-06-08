use super::super::super::*;

impl App {
    pub fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
        if self.settings_open {
            self.mode = Mode::Settings;
            self.reset_settings_list_state(false);
        } else {
            self.close_settings();
            return;
        }
        self.dirty = true;
    }

    pub fn open_settings_search(&mut self) {
        self.settings_open = true;
        self.mode = Mode::Settings;
        self.reset_settings_list_state(true);
        self.dirty = true;
    }

    pub fn close_settings(&mut self) {
        self.restore_settings_detail_preview_state();
        self.settings_open = false;
        self.theme_selector_open = false;
        self.settings_focus = SettingsFocus::List;
        self.active_settings_detail = None;
        self.settings_searching = false;
        self.settings_search.clear();
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    fn reset_settings_list_state(&mut self, searching: bool) {
        self.theme_selector_open = false;
        self.settings_selected = 0;
        self.settings_focus = SettingsFocus::List;
        self.active_settings_detail = None;
        self.settings_searching = searching;
        self.settings_search.clear();
    }
}
