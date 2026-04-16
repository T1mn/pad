use super::*;

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub status_fg: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub comment: Color,
    pub keyword: Color,
    pub string_color: Color,
    pub number: Color,
    pub mode_normal_bg: Color,
    pub mode_search_bg: Color,
    pub mode_tree_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::by_name("default")
    }
}

impl Theme {
    pub fn by_name(name: &str) -> Self {
        let theme = match name {
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            "catppuccin" => Self::catppuccin(),
            "gruvbox" => Self::gruvbox(),
            "tokyo-night" => Self::tokyo_night(),
            "monokai" => Self::monokai(),
            "solarized-dark" => Self::solarized_dark(),
            "rose-pine" => Self::rose_pine(),
            "solarized-light" => Self::solarized_light(),
            "one-dark" => Self::one_dark(),
            "github-light" => Self::github_light(),
            "github-dark" => Self::github_dark(),
            "everforest" => Self::everforest(),
            "dark" => Self::dark(),
            _ => Self::default_theme(),
        };

        theme.with_readability_boost()
    }

    fn with_readability_boost(mut self) -> Self {
        self.highlight_fg = super::color::readable_text_color(self.fg, self.highlight_fg, 0.62);
        self.status_fg = super::color::readable_text_color(self.fg, self.status_fg, 0.82);
        self.comment = super::color::readable_text_color(self.fg, self.comment, 0.38);
        self.border = super::color::readable_text_color(self.fg, self.border, 0.22);
        self.highlight_bg = super::color::readable_surface_color(self.fg, self.highlight_bg, 0.12);
        self
    }
}
