use super::super::*;

impl App {
    #[allow(dead_code)]
    pub fn open_theme_selector(&mut self) {
        self.preview.theme_before_preview = Some(self.config.theme.clone());
        self.theme_selector_open = true;
        self.mode = Mode::ThemeSelector;
        self.theme_selected = 0;
        self.dirty = true;
    }

    pub fn close_theme_selector(&mut self) {
        if let Some(ref prev) = self.preview.theme_before_preview.take() {
            self.theme = crate::theme::Theme::by_name(prev);
        }
        self.theme_selector_open = false;
        self.mode = Mode::Settings;
        self.dirty = true;
    }

    pub fn available_locales() -> Vec<crate::i18n::Locale> {
        use crate::i18n::Locale;
        vec![
            Locale::En,
            Locale::ZhCN,
            Locale::ZhTW,
            Locale::Ja,
            Locale::De,
            Locale::Fr,
        ]
    }

    #[allow(dead_code)]
    pub fn open_language_selector(&mut self) {
        let locales = Self::available_locales();
        self.language_selected = locales.iter().position(|l| *l == self.locale).unwrap_or(0);
        self.mode = Mode::LanguageSelector;
        self.dirty = true;
    }

    pub fn close_language_selector(&mut self) {
        self.locale = crate::i18n::Locale::from_str(&self.config.language);
        self.mode = Mode::Settings;
        self.dirty = true;
    }

    pub fn available_themes() -> Vec<(&'static str, &'static str)> {
        vec![
            ("default", "Default"),
            ("dark", "Dark"),
            ("dracula", "Dracula"),
            ("nord", "Nord"),
            ("gruvbox", "Gruvbox"),
            ("catppuccin", "Catppuccin"),
            ("tokyo-night", "Tokyo Night"),
            ("monokai", "Monokai"),
            ("solarized-dark", "Solarized Dark"),
            ("solarized-light", "Solarized Light"),
            ("rose-pine", "Rose Pine"),
            ("one-dark", "One Dark"),
            ("github-light", "GitHub Light"),
            ("github-dark", "GitHub Dark"),
            ("everforest", "Everforest"),
        ]
    }
}
