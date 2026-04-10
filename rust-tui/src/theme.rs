use ratatui::style::Color;
use reqwest::Url;
use std::collections::HashMap;
use std::path::PathBuf;

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
        self.highlight_fg = readable_text_color(self.fg, self.highlight_fg, 0.62);
        self.status_fg = readable_text_color(self.fg, self.status_fg, 0.82);
        self.comment = readable_text_color(self.fg, self.comment, 0.38);
        self.border = readable_text_color(self.fg, self.border, 0.22);
        self.highlight_bg = readable_surface_color(self.fg, self.highlight_bg, 0.12);
        self
    }

    fn default_theme() -> Self {
        Self {
            name: "default",
            bg: Color::Reset,
            fg: Color::Reset,
            accent: Color::Cyan,
            highlight_bg: Color::DarkGray,
            highlight_fg: Color::White,
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            status_fg: Color::White,
            error: Color::Red,
            success: Color::Green,
            warning: Color::Yellow,
            comment: Color::DarkGray,
            keyword: Color::Magenta,
            string_color: Color::Green,
            number: Color::Cyan,
            mode_normal_bg: Color::Blue,
            mode_search_bg: Color::Yellow,
            mode_tree_bg: Color::Green,
        }
    }

    fn dark() -> Self {
        Self {
            name: "dark",
            bg: Color::Rgb(30, 30, 30),
            fg: Color::Rgb(204, 204, 204),
            accent: Color::Rgb(86, 182, 194),
            highlight_bg: Color::Rgb(60, 60, 60),
            highlight_fg: Color::White,
            border: Color::Rgb(68, 68, 68),
            border_focused: Color::Rgb(86, 182, 194),
            status_fg: Color::Rgb(204, 204, 204),
            error: Color::Rgb(244, 71, 71),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            comment: Color::Rgb(92, 99, 112),
            keyword: Color::Rgb(198, 120, 221),
            string_color: Color::Rgb(152, 195, 121),
            number: Color::Rgb(209, 154, 102),
            mode_normal_bg: Color::Rgb(86, 182, 194),
            mode_search_bg: Color::Rgb(229, 192, 123),
            mode_tree_bg: Color::Rgb(152, 195, 121),
        }
    }

    fn dracula() -> Self {
        Self {
            name: "dracula",
            bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            accent: Color::Rgb(189, 147, 249),
            highlight_bg: Color::Rgb(68, 71, 90),
            highlight_fg: Color::Rgb(248, 248, 242),
            border: Color::Rgb(68, 71, 90),
            border_focused: Color::Rgb(189, 147, 249),
            status_fg: Color::Rgb(248, 248, 242),
            error: Color::Rgb(255, 85, 85),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            comment: Color::Rgb(98, 114, 164),
            keyword: Color::Rgb(255, 121, 198),
            string_color: Color::Rgb(241, 250, 140),
            number: Color::Rgb(189, 147, 249),
            mode_normal_bg: Color::Rgb(189, 147, 249),
            mode_search_bg: Color::Rgb(241, 250, 140),
            mode_tree_bg: Color::Rgb(80, 250, 123),
        }
    }

    fn nord() -> Self {
        Self {
            name: "nord",
            bg: Color::Rgb(46, 52, 64),
            fg: Color::Rgb(216, 222, 233),
            accent: Color::Rgb(136, 192, 208),
            highlight_bg: Color::Rgb(67, 76, 94),
            highlight_fg: Color::Rgb(236, 239, 244),
            border: Color::Rgb(59, 66, 82),
            border_focused: Color::Rgb(136, 192, 208),
            status_fg: Color::Rgb(216, 222, 233),
            error: Color::Rgb(191, 97, 106),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            comment: Color::Rgb(76, 86, 106),
            keyword: Color::Rgb(180, 142, 173),
            string_color: Color::Rgb(163, 190, 140),
            number: Color::Rgb(180, 142, 173),
            mode_normal_bg: Color::Rgb(136, 192, 208),
            mode_search_bg: Color::Rgb(235, 203, 139),
            mode_tree_bg: Color::Rgb(163, 190, 140),
        }
    }

    fn catppuccin() -> Self {
        Self {
            name: "catppuccin",
            bg: Color::Rgb(30, 30, 46),
            fg: Color::Rgb(205, 214, 244),
            accent: Color::Rgb(137, 180, 250),
            highlight_bg: Color::Rgb(49, 50, 68),
            highlight_fg: Color::Rgb(205, 214, 244),
            border: Color::Rgb(69, 71, 90),
            border_focused: Color::Rgb(137, 180, 250),
            status_fg: Color::Rgb(205, 214, 244),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            comment: Color::Rgb(108, 112, 134),
            keyword: Color::Rgb(203, 166, 247),
            string_color: Color::Rgb(166, 227, 161),
            number: Color::Rgb(250, 179, 135),
            mode_normal_bg: Color::Rgb(137, 180, 250),
            mode_search_bg: Color::Rgb(249, 226, 175),
            mode_tree_bg: Color::Rgb(166, 227, 161),
        }
    }

    fn gruvbox() -> Self {
        Self {
            name: "gruvbox",
            bg: Color::Rgb(40, 40, 40),
            fg: Color::Rgb(235, 219, 178),
            accent: Color::Rgb(131, 165, 152),
            highlight_bg: Color::Rgb(80, 73, 69),
            highlight_fg: Color::Rgb(251, 241, 199),
            border: Color::Rgb(60, 56, 54),
            border_focused: Color::Rgb(131, 165, 152),
            status_fg: Color::Rgb(235, 219, 178),
            error: Color::Rgb(251, 73, 52),
            success: Color::Rgb(184, 187, 38),
            warning: Color::Rgb(250, 189, 47),
            comment: Color::Rgb(146, 131, 116),
            keyword: Color::Rgb(211, 134, 155),
            string_color: Color::Rgb(184, 187, 38),
            number: Color::Rgb(211, 134, 155),
            mode_normal_bg: Color::Rgb(131, 165, 152),
            mode_search_bg: Color::Rgb(250, 189, 47),
            mode_tree_bg: Color::Rgb(184, 187, 38),
        }
    }

    fn tokyo_night() -> Self {
        Self {
            name: "tokyo-night",
            bg: Color::Rgb(26, 27, 38),
            fg: Color::Rgb(169, 177, 214),
            accent: Color::Rgb(122, 162, 247),
            highlight_bg: Color::Rgb(41, 46, 66),
            highlight_fg: Color::Rgb(192, 202, 245),
            border: Color::Rgb(41, 46, 66),
            border_focused: Color::Rgb(122, 162, 247),
            status_fg: Color::Rgb(169, 177, 214),
            error: Color::Rgb(247, 118, 142),
            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            comment: Color::Rgb(86, 95, 137),
            keyword: Color::Rgb(187, 154, 247),
            string_color: Color::Rgb(158, 206, 106),
            number: Color::Rgb(255, 158, 100),
            mode_normal_bg: Color::Rgb(122, 162, 247),
            mode_search_bg: Color::Rgb(224, 175, 104),
            mode_tree_bg: Color::Rgb(158, 206, 106),
        }
    }

    fn monokai() -> Self {
        Self {
            name: "monokai",
            bg: Color::Rgb(39, 40, 34),
            fg: Color::Rgb(248, 248, 242),
            accent: Color::Rgb(102, 217, 239),
            highlight_bg: Color::Rgb(73, 72, 62),
            highlight_fg: Color::Rgb(248, 248, 242),
            border: Color::Rgb(73, 72, 62),
            border_focused: Color::Rgb(102, 217, 239),
            status_fg: Color::Rgb(248, 248, 242),
            error: Color::Rgb(249, 38, 114),
            success: Color::Rgb(166, 226, 46),
            warning: Color::Rgb(253, 151, 31),
            comment: Color::Rgb(117, 113, 94),
            keyword: Color::Rgb(249, 38, 114),
            string_color: Color::Rgb(230, 219, 116),
            number: Color::Rgb(174, 129, 255),
            mode_normal_bg: Color::Rgb(102, 217, 239),
            mode_search_bg: Color::Rgb(253, 151, 31),
            mode_tree_bg: Color::Rgb(166, 226, 46),
        }
    }

    fn solarized_dark() -> Self {
        Self {
            name: "solarized-dark",
            bg: Color::Rgb(0, 43, 54),
            fg: Color::Rgb(131, 148, 150),
            accent: Color::Rgb(38, 139, 210),
            highlight_bg: Color::Rgb(7, 54, 66),
            highlight_fg: Color::Rgb(147, 161, 161),
            border: Color::Rgb(7, 54, 66),
            border_focused: Color::Rgb(38, 139, 210),
            status_fg: Color::Rgb(131, 148, 150),
            error: Color::Rgb(220, 50, 47),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(181, 137, 0),
            comment: Color::Rgb(88, 110, 117),
            keyword: Color::Rgb(108, 113, 196),
            string_color: Color::Rgb(42, 161, 152),
            number: Color::Rgb(203, 75, 22),
            mode_normal_bg: Color::Rgb(38, 139, 210),
            mode_search_bg: Color::Rgb(181, 137, 0),
            mode_tree_bg: Color::Rgb(133, 153, 0),
        }
    }

    fn rose_pine() -> Self {
        Self {
            name: "rose-pine",
            bg: Color::Rgb(25, 23, 36),
            fg: Color::Rgb(224, 222, 244),
            accent: Color::Rgb(196, 167, 231),
            highlight_bg: Color::Rgb(38, 35, 53),
            highlight_fg: Color::Rgb(224, 222, 244),
            border: Color::Rgb(38, 35, 53),
            border_focused: Color::Rgb(196, 167, 231),
            status_fg: Color::Rgb(224, 222, 244),
            error: Color::Rgb(235, 111, 146),
            success: Color::Rgb(156, 207, 216),
            warning: Color::Rgb(246, 193, 119),
            comment: Color::Rgb(110, 106, 134),
            keyword: Color::Rgb(196, 167, 231),
            string_color: Color::Rgb(246, 193, 119),
            number: Color::Rgb(235, 188, 186),
            mode_normal_bg: Color::Rgb(196, 167, 231),
            mode_search_bg: Color::Rgb(246, 193, 119),
            mode_tree_bg: Color::Rgb(156, 207, 216),
        }
    }

    fn solarized_light() -> Self {
        Self {
            name: "solarized-light",
            bg: Color::Rgb(253, 246, 227),
            fg: Color::Rgb(88, 110, 117),
            accent: Color::Rgb(38, 139, 210),
            highlight_bg: Color::Rgb(238, 232, 213),
            highlight_fg: Color::Rgb(7, 54, 66),
            border: Color::Rgb(147, 161, 161),
            border_focused: Color::Rgb(38, 139, 210),
            status_fg: Color::Rgb(131, 148, 150),
            error: Color::Rgb(220, 50, 47),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(181, 137, 0),
            comment: Color::Rgb(131, 148, 150),
            keyword: Color::Rgb(108, 113, 196),
            string_color: Color::Rgb(42, 161, 152),
            number: Color::Rgb(211, 54, 130),
            mode_normal_bg: Color::Rgb(38, 139, 210),
            mode_search_bg: Color::Rgb(181, 137, 0),
            mode_tree_bg: Color::Rgb(133, 153, 0),
        }
    }

    fn one_dark() -> Self {
        Self {
            name: "one-dark",
            bg: Color::Rgb(40, 44, 52),
            fg: Color::Rgb(171, 178, 191),
            accent: Color::Rgb(97, 175, 239),
            highlight_bg: Color::Rgb(62, 68, 81),
            highlight_fg: Color::Rgb(229, 192, 123),
            border: Color::Rgb(92, 99, 112),
            border_focused: Color::Rgb(97, 175, 239),
            status_fg: Color::Rgb(92, 99, 112),
            error: Color::Rgb(224, 108, 117),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            comment: Color::Rgb(92, 99, 112),
            keyword: Color::Rgb(198, 120, 221),
            string_color: Color::Rgb(152, 195, 121),
            number: Color::Rgb(209, 154, 102),
            mode_normal_bg: Color::Rgb(97, 175, 239),
            mode_search_bg: Color::Rgb(229, 192, 123),
            mode_tree_bg: Color::Rgb(152, 195, 121),
        }
    }

    fn github_light() -> Self {
        Self {
            name: "github-light",
            bg: Color::Rgb(255, 255, 255),
            fg: Color::Rgb(36, 41, 46),
            accent: Color::Rgb(3, 102, 214),
            highlight_bg: Color::Rgb(241, 248, 255),
            highlight_fg: Color::Rgb(36, 41, 46),
            border: Color::Rgb(225, 228, 232),
            border_focused: Color::Rgb(3, 102, 214),
            status_fg: Color::Rgb(88, 96, 105),
            error: Color::Rgb(215, 58, 73),
            success: Color::Rgb(40, 167, 69),
            warning: Color::Rgb(249, 130, 108),
            comment: Color::Rgb(106, 115, 125),
            keyword: Color::Rgb(215, 58, 73),
            string_color: Color::Rgb(3, 47, 98),
            number: Color::Rgb(0, 92, 197),
            mode_normal_bg: Color::Rgb(3, 102, 214),
            mode_search_bg: Color::Rgb(249, 130, 108),
            mode_tree_bg: Color::Rgb(40, 167, 69),
        }
    }

    fn github_dark() -> Self {
        Self {
            name: "github-dark",
            bg: Color::Rgb(13, 17, 23),
            fg: Color::Rgb(201, 209, 217),
            accent: Color::Rgb(88, 166, 255),
            highlight_bg: Color::Rgb(22, 27, 34),
            highlight_fg: Color::Rgb(240, 246, 252),
            border: Color::Rgb(48, 54, 61),
            border_focused: Color::Rgb(88, 166, 255),
            status_fg: Color::Rgb(139, 148, 158),
            error: Color::Rgb(248, 81, 73),
            success: Color::Rgb(63, 185, 80),
            warning: Color::Rgb(210, 153, 34),
            comment: Color::Rgb(139, 148, 158),
            keyword: Color::Rgb(255, 123, 114),
            string_color: Color::Rgb(165, 214, 255),
            number: Color::Rgb(121, 192, 255),
            mode_normal_bg: Color::Rgb(88, 166, 255),
            mode_search_bg: Color::Rgb(210, 153, 34),
            mode_tree_bg: Color::Rgb(63, 185, 80),
        }
    }

    fn everforest() -> Self {
        Self {
            name: "everforest",
            bg: Color::Rgb(45, 53, 59),
            fg: Color::Rgb(211, 198, 170),
            accent: Color::Rgb(167, 192, 128),
            highlight_bg: Color::Rgb(61, 72, 77),
            highlight_fg: Color::Rgb(211, 198, 170),
            border: Color::Rgb(71, 82, 88),
            border_focused: Color::Rgb(167, 192, 128),
            status_fg: Color::Rgb(133, 146, 137),
            error: Color::Rgb(230, 126, 128),
            success: Color::Rgb(167, 192, 128),
            warning: Color::Rgb(219, 188, 127),
            comment: Color::Rgb(133, 146, 137),
            keyword: Color::Rgb(214, 153, 182),
            string_color: Color::Rgb(167, 192, 128),
            number: Color::Rgb(214, 153, 182),
            mode_normal_bg: Color::Rgb(167, 192, 128),
            mode_search_bg: Color::Rgb(219, 188, 127),
            mode_tree_bg: Color::Rgb(131, 192, 146),
        }
    }
}

/// Per-provider relay configuration
#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub label: String,
    pub base_url: String,
    pub api_key: String,
    pub env_key: String,
    pub wire_api: String,
    pub provider_key: String,
    pub npm_package: String,
    pub models: Vec<OpenCodeModelConfig>,
    /// Runtime-only: None=untested, Some(true)=ok, Some(false)=failed
    pub test_status: Option<bool>,
    /// Runtime-only: last HTTP status returned by the lightweight connectivity probe
    pub test_http_status: Option<u16>,
    /// Runtime-only: last measured latency in milliseconds
    pub test_latency_ms: Option<u64>,
    /// Runtime-only: human-readable test result (model list or error)
    pub test_result: Option<String>,
}

impl ProviderConfig {
    pub fn codex_provider_name(&self) -> String {
        normalize_provider_name(&self.label)
    }

    pub fn codex_auth_token(&self) -> Option<String> {
        if !self.api_key.trim().is_empty() {
            return Some(self.api_key.clone());
        }
        None
    }

    pub fn codex_wire_api(&self) -> &str {
        if self.wire_api.trim().is_empty() {
            "responses"
        } else {
            self.wire_api.as_str()
        }
    }

    pub fn codex_base_url(&self) -> String {
        codex_preferred_api_base_url(&self.base_url)
    }

    pub fn opencode_provider_key(&self) -> &str {
        if self.provider_key.trim().is_empty() {
            "provider"
        } else {
            self.provider_key.as_str()
        }
    }

    pub fn opencode_npm_package(&self) -> &str {
        if self.npm_package.trim().is_empty() {
            "@ai-sdk/openai-compatible"
        } else {
            self.npm_package.as_str()
        }
    }
}

fn normalize_provider_name(raw: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_sep = false;

    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !normalized.is_empty() && !last_was_sep {
            normalized.push('_');
            last_was_sep = true;
        }
    }

    let normalized = normalized.trim_matches('_').to_string();
    if normalized.is_empty() {
        "relay".to_string()
    } else {
        normalized
    }
}

fn trim_base_url(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

fn readable_text_color(primary: Color, current: Color, mix: f32) -> Color {
    blend_theme_color(primary, current, mix)
}

fn readable_surface_color(primary: Color, current: Color, mix: f32) -> Color {
    blend_theme_color(primary, current, mix)
}

fn blend_theme_color(target: Color, base: Color, mix: f32) -> Color {
    let mix = mix.clamp(0.0, 1.0);
    match (theme_rgb(target), theme_rgb(base)) {
        (Some((tr, tg, tb)), Some((br, bg, bb))) => Color::Rgb(
            blend_theme_channel(tr, br, mix),
            blend_theme_channel(tg, bg, mix),
            blend_theme_channel(tb, bb, mix),
        ),
        _ if mix >= 0.5 => target,
        _ => base,
    }
}

fn blend_theme_channel(target: u8, base: u8, mix: f32) -> u8 {
    let target = target as f32;
    let base = base as f32;
    (base + (target - base) * mix).round().clamp(0.0, 255.0) as u8
}

fn theme_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((255, 0, 0)),
        Color::Green => Some((0, 128, 0)),
        Color::Yellow => Some((255, 255, 0)),
        Color::Blue => Some((0, 0, 255)),
        Color::Magenta => Some((255, 0, 255)),
        Color::Cyan => Some((0, 255, 255)),
        Color::Gray => Some((128, 128, 128)),
        Color::DarkGray => Some((64, 64, 64)),
        Color::LightRed => Some((255, 102, 102)),
        Color::LightGreen => Some((144, 238, 144)),
        Color::LightYellow => Some((255, 255, 224)),
        Color::LightBlue => Some((173, 216, 230)),
        Color::LightMagenta => Some((255, 153, 255)),
        Color::LightCyan => Some((224, 255, 255)),
        Color::White => Some((255, 255, 255)),
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(_) | Color::Reset => None,
    }
}

fn push_unique_url(out: &mut Vec<String>, value: String) {
    if !value.is_empty() && !out.iter().any(|existing| existing == &value) {
        out.push(value);
    }
}

pub(crate) fn codex_api_base_candidates(raw: &str) -> Vec<String> {
    let trimmed = trim_base_url(raw);
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    push_unique_url(&mut candidates, trimmed.clone());

    let Ok(mut parsed) = Url::parse(&trimmed) else {
        return candidates;
    };

    let path = parsed.path().trim_end_matches('/');
    if path.is_empty() || path == "/" {
        parsed.set_path("/v1");
        push_unique_url(&mut candidates, trim_base_url(parsed.as_str()));
    } else if path == "/v1" {
        parsed.set_path("");
        push_unique_url(&mut candidates, trim_base_url(parsed.as_str()));
    }

    candidates
}

pub(crate) fn codex_preferred_api_base_url(raw: &str) -> String {
    let candidates = codex_api_base_candidates(raw);
    if candidates.is_empty() {
        return String::new();
    }

    let trimmed = trim_base_url(raw);
    if let Ok(parsed) = Url::parse(&trimmed) {
        let path = parsed.path().trim_end_matches('/');
        if path.is_empty() || path == "/" {
            if let Some(candidate) = candidates
                .iter()
                .find(|candidate| candidate.ends_with("/v1"))
            {
                return candidate.clone();
            }
        }
    }

    candidates[0].clone()
}

pub fn normalize_provider_key(raw: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_sep = false;

    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !normalized.is_empty() && !last_was_sep {
            normalized.push('-');
            last_was_sep = true;
        }
    }

    let normalized = normalized.trim_matches('-').to_string();
    if normalized.is_empty() {
        "provider".to_string()
    } else {
        normalized
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenCodeModelConfig {
    pub id: String,
    pub name: String,
}

/// Per-agent configuration including relay/proxy settings
#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub name: String,
    pub cmd: String,
    pub providers: Vec<ProviderConfig>,
    pub active_provider: Option<usize>,
    pub default_model: String,
    pub small_model: String,
    // Legacy fields kept for backward compat during migration
    #[allow(dead_code)]
    pub base_url: Option<String>,
    #[allow(dead_code)]
    pub api_key: Option<String>,
}

impl AgentConfig {
    /// Get the currently active provider, if any
    pub fn active(&self) -> Option<&ProviderConfig> {
        self.active_provider.and_then(|i| self.providers.get(i))
    }

    pub fn opencode_model_options(&self) -> Vec<(String, String)> {
        let mut options = Vec::new();
        for provider in &self.providers {
            let provider_key = provider.opencode_provider_key();
            let provider_label = if provider.label.trim().is_empty() {
                provider_key
            } else {
                provider.label.as_str()
            };
            for model in &provider.models {
                if model.id.trim().is_empty() {
                    continue;
                }
                let value = format!("{provider_key}/{}", model.id.trim());
                let model_name = if model.name.trim().is_empty() {
                    model.id.trim()
                } else {
                    model.name.trim()
                };
                let label = format!("{provider_label} / {model_name} ({})", model.id.trim());
                options.push((value, label));
            }
        }
        options
    }

    pub fn opencode_first_model_value(&self) -> Option<String> {
        self.opencode_model_options()
            .into_iter()
            .next()
            .map(|(value, _)| value)
    }

    pub fn repair_opencode_model_refs(&mut self) {
        let valid: std::collections::HashSet<String> = self
            .opencode_model_options()
            .into_iter()
            .map(|(value, _)| value)
            .collect();

        if !self.default_model.is_empty() && !valid.contains(&self.default_model) {
            self.default_model = self.opencode_first_model_value().unwrap_or_default();
        }

        if !self.small_model.is_empty() && !valid.contains(&self.small_model) {
            self.small_model.clear();
        }
    }

    pub fn rename_opencode_provider_key(&mut self, old_key: &str, new_key: &str) {
        if old_key == new_key {
            return;
        }
        if self.default_model.starts_with(&format!("{old_key}/")) {
            self.default_model = self.default_model.replacen(old_key, new_key, 1);
        }
        if self.small_model.starts_with(&format!("{old_key}/")) {
            self.small_model = self.small_model.replacen(old_key, new_key, 1);
        }
    }

    pub fn rename_opencode_model_id(&mut self, provider_key: &str, old_id: &str, new_id: &str) {
        let old_value = format!("{provider_key}/{old_id}");
        let new_value = format!("{provider_key}/{new_id}");
        if self.default_model == old_value {
            self.default_model = new_value.clone();
        }
        if self.small_model == old_value {
            self.small_model = new_value;
        }
    }
}

/// Desired state for the agent pane when attaching
#[derive(Clone, Debug, PartialEq)]
pub struct DesiredAgentStyle {
    /// "auto" = zoom if multi-pane and not already zoomed; "keep" = do nothing
    pub zoom: String,
    /// "show" = force status on; "hide" = force status off; "keep" = do nothing
    pub status: String,
}

impl Default for DesiredAgentStyle {
    fn default() -> Self {
        Self {
            zoom: "auto".to_string(),
            status: "show".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewConfig {
    /// "auto" = Claude/Codex prefer session preview, fallback to tmux
    /// "tmux" = always use tmux capture-pane
    /// "session" = always use session transcript when available
    pub mode: String,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            mode: "auto".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
    pub bot_username: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentPermissionsConfig {
    pub codex_auto_full_access: bool,
    pub claude_auto_full_access: bool,
}

impl Default for AgentPermissionsConfig {
    fn default() -> Self {
        Self {
            codex_auto_full_access: true,
            claude_auto_full_access: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DisplayConfig {
    pub session_scope: String,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            session_scope: "live".to_string(),
        }
    }
}

/// Config file management
#[derive(Clone, Debug)]
pub struct Config {
    pub theme: String,
    pub auto_refresh: bool,
    pub refresh_interval: u64,
    pub agents: Vec<AgentConfig>,
    pub language: String,
    pub desired_agent_style: DesiredAgentStyle,
    pub preview: PreviewConfig,
    pub display: DisplayConfig,
    pub telegram: TelegramConfig,
    pub agent_permissions: AgentPermissionsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            auto_refresh: true,
            refresh_interval: 10,
            agents: vec![
                AgentConfig {
                    name: "claude".into(),
                    cmd: "claude".into(),
                    providers: Vec::new(),
                    active_provider: None,
                    default_model: String::new(),
                    small_model: String::new(),
                    base_url: None,
                    api_key: None,
                },
                AgentConfig {
                    name: "codex".into(),
                    cmd: "codex".into(),
                    providers: Vec::new(),
                    active_provider: None,
                    default_model: String::new(),
                    small_model: String::new(),
                    base_url: None,
                    api_key: None,
                },
                AgentConfig {
                    name: "gemini".into(),
                    cmd: "gemini".into(),
                    providers: Vec::new(),
                    active_provider: None,
                    default_model: String::new(),
                    small_model: String::new(),
                    base_url: None,
                    api_key: None,
                },
                AgentConfig {
                    name: "opencode".into(),
                    cmd: "opencode".into(),
                    providers: Vec::new(),
                    active_provider: None,
                    default_model: String::new(),
                    small_model: String::new(),
                    base_url: None,
                    api_key: None,
                },
            ],
            language: "en".to_string(),
            desired_agent_style: DesiredAgentStyle::default(),
            preview: PreviewConfig::default(),
            display: DisplayConfig::default(),
            telegram: TelegramConfig::default(),
            agent_permissions: AgentPermissionsConfig::default(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        crate::paths::config_path()
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let legacy_path = crate::paths::legacy_config_path();
        let load_path = if path.exists() {
            path
        } else if legacy_path.exists() {
            legacy_path
        } else {
            return Self::default();
        };

        let content = match std::fs::read_to_string(&load_path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let table: HashMap<String, toml::Value> = match toml::from_str(&content) {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };

        let mut config = Self::default();

        if let Some(toml::Value::String(theme)) = table.get("theme") {
            config.theme = theme.clone();
        }
        if let Some(toml::Value::Boolean(auto)) = table.get("auto_refresh") {
            config.auto_refresh = *auto;
        }
        if let Some(toml::Value::Integer(interval)) = table.get("refresh_interval") {
            config.refresh_interval = *interval as u64;
        }
        if let Some(toml::Value::String(lang)) = table.get("language") {
            config.language = lang.clone();
        }
        if let Some(toml::Value::String(sb)) = table.get("status_bar") {
            config.desired_agent_style.status = match sb.as_str() {
                "hidden" => "hide".to_string(),
                "show" => "show".to_string(),
                other => other.to_string(),
            };
        }
        if let Some(toml::Value::Table(das)) = table.get("desired_agent_style") {
            if let Some(toml::Value::String(z)) = das.get("zoom") {
                config.desired_agent_style.zoom = z.clone();
            }
            if let Some(toml::Value::String(s)) = das.get("status") {
                config.desired_agent_style.status = s.clone();
            }
        }
        if let Some(toml::Value::Table(preview)) = table.get("preview") {
            if let Some(toml::Value::String(mode)) = preview.get("mode") {
                config.preview.mode = match mode.as_str() {
                    "tmux" => "tmux".to_string(),
                    "session" => "session".to_string(),
                    _ => "auto".to_string(),
                };
            }
        }
        if let Some(toml::Value::Table(display)) = table.get("display") {
            if let Some(toml::Value::String(scope)) = display.get("session_scope") {
                config.display.session_scope = match scope.as_str() {
                    "all" => "all".to_string(),
                    _ => "live".to_string(),
                };
            }
        }
        if let Some(toml::Value::Table(telegram)) = table.get("telegram") {
            if let Some(toml::Value::Boolean(enabled)) = telegram.get("enabled") {
                config.telegram.enabled = *enabled;
            }
            if let Some(toml::Value::String(token)) = telegram.get("bot_token") {
                config.telegram.bot_token = token.clone();
            }
            if let Some(toml::Value::String(chat_id)) = telegram.get("chat_id") {
                config.telegram.chat_id = chat_id.clone();
            }
            if let Some(toml::Value::String(bot_username)) = telegram.get("bot_username") {
                config.telegram.bot_username = bot_username.clone();
            }
        }
        if let Some(toml::Value::Table(agent_permissions)) = table.get("agent_permissions") {
            if let Some(toml::Value::Boolean(enabled)) =
                agent_permissions.get("codex_auto_full_access")
            {
                config.agent_permissions.codex_auto_full_access = *enabled;
            }
            if let Some(toml::Value::Boolean(enabled)) =
                agent_permissions.get("claude_auto_full_access")
            {
                config.agent_permissions.claude_auto_full_access = *enabled;
            }
        }
        if let Some(toml::Value::Array(agents)) = table.get("agents") {
            let mut parsed = Vec::new();
            for agent in agents {
                if let toml::Value::Table(t) = agent {
                    if let (Some(toml::Value::String(name)), Some(toml::Value::String(cmd))) =
                        (t.get("name"), t.get("cmd"))
                    {
                        // Parse providers array if present
                        let mut providers = Vec::new();
                        let mut active_provider = None;
                        let mut default_model = t
                            .get("default_model")
                            .and_then(|v| {
                                if let toml::Value::String(s) = v {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default();
                        let mut small_model = t
                            .get("small_model")
                            .and_then(|v| {
                                if let toml::Value::String(s) = v {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default();
                        if let Some(toml::Value::Array(provs)) = t.get("providers") {
                            for prov in provs {
                                if let toml::Value::Table(pt) = prov {
                                    let label = pt
                                        .get("label")
                                        .and_then(|v| {
                                            if let toml::Value::String(s) = v {
                                                Some(s.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or_default();
                                    let base_url = pt
                                        .get("base_url")
                                        .and_then(|v| {
                                            if let toml::Value::String(s) = v {
                                                Some(s.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or_default();
                                    let api_key = pt
                                        .get("api_key")
                                        .and_then(|v| {
                                            if let toml::Value::String(s) = v {
                                                Some(s.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or_default();
                                    let env_key = pt
                                        .get("env_key")
                                        .and_then(|v| {
                                            if let toml::Value::String(s) = v {
                                                Some(s.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or_default();
                                    let wire_api = pt
                                        .get("wire_api")
                                        .and_then(|v| {
                                            if let toml::Value::String(s) = v {
                                                Some(s.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or_else(|| "responses".to_string());
                                    let provider_key = pt
                                        .get("provider_key")
                                        .and_then(|v| {
                                            if let toml::Value::String(s) = v {
                                                Some(s.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or_else(|| normalize_provider_key(&label));
                                    let npm_package = pt
                                        .get("npm_package")
                                        .and_then(|v| {
                                            if let toml::Value::String(s) = v {
                                                Some(s.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or_else(|| "@ai-sdk/openai-compatible".to_string());
                                    let models = pt
                                        .get("models")
                                        .and_then(|v| v.as_array())
                                        .map(|items| {
                                            items
                                                .iter()
                                                .filter_map(|item| {
                                                    let table = item.as_table()?;
                                                    let id = table
                                                        .get("id")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or_default()
                                                        .to_string();
                                                    let name = table
                                                        .get("name")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or_default()
                                                        .to_string();
                                                    if id.trim().is_empty() {
                                                        None
                                                    } else {
                                                        Some(OpenCodeModelConfig { id, name })
                                                    }
                                                })
                                                .collect::<Vec<_>>()
                                        })
                                        .unwrap_or_default();
                                    providers.push(ProviderConfig {
                                        label,
                                        base_url,
                                        api_key,
                                        env_key,
                                        wire_api,
                                        provider_key,
                                        npm_package,
                                        models,
                                        test_status: None,
                                        test_http_status: None,
                                        test_latency_ms: None,
                                        test_result: None,
                                    });
                                }
                            }
                        }
                        if let Some(toml::Value::Integer(idx)) = t.get("active_provider") {
                            let idx = *idx as usize;
                            if idx < providers.len() {
                                active_provider = Some(idx);
                            }
                        }

                        // Legacy migration: old base_url/api_key -> single provider
                        let legacy_url = t.get("base_url").and_then(|v| {
                            if let toml::Value::String(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        });
                        let legacy_key = t.get("api_key").and_then(|v| {
                            if let toml::Value::String(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        });
                        if providers.is_empty() && (legacy_url.is_some() || legacy_key.is_some()) {
                            providers.push(ProviderConfig {
                                label: "default".to_string(),
                                base_url: legacy_url.clone().unwrap_or_default(),
                                api_key: legacy_key.clone().unwrap_or_default(),
                                env_key: String::new(),
                                wire_api: "responses".to_string(),
                                provider_key: "default".to_string(),
                                npm_package: "@ai-sdk/openai-compatible".to_string(),
                                models: Vec::new(),
                                test_status: None,
                                test_http_status: None,
                                test_latency_ms: None,
                                test_result: None,
                            });
                            active_provider = Some(0);
                        }

                        let mut parsed_agent = AgentConfig {
                            name: name.clone(),
                            cmd: cmd.clone(),
                            providers,
                            active_provider,
                            default_model: std::mem::take(&mut default_model),
                            small_model: std::mem::take(&mut small_model),
                            base_url: legacy_url,
                            api_key: legacy_key,
                        };
                        if parsed_agent.name == "opencode" {
                            parsed_agent.repair_opencode_model_refs();
                        }
                        parsed.push(parsed_agent);
                    }
                }
            }
            if !parsed.is_empty() {
                if !parsed.iter().any(|agent| agent.name == "opencode") {
                    parsed.push(AgentConfig {
                        name: "opencode".into(),
                        cmd: "opencode".into(),
                        providers: Vec::new(),
                        active_provider: None,
                        default_model: String::new(),
                        small_model: String::new(),
                        base_url: None,
                        api_key: None,
                    });
                }
                config.agents = parsed;
            }
        }

        config
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut content = String::new();
        content.push_str(&format!("theme = \"{}\"\n", self.theme));
        content.push_str(&format!("auto_refresh = {}\n", self.auto_refresh));
        content.push_str(&format!("refresh_interval = {}\n", self.refresh_interval));
        content.push_str(&format!("language = \"{}\"\n", self.language));
        content.push_str("\n[desired_agent_style]\n");
        content.push_str(&format!("zoom = \"{}\"\n", self.desired_agent_style.zoom));
        content.push_str(&format!(
            "status = \"{}\"\n",
            self.desired_agent_style.status
        ));
        content.push_str("\n[preview]\n");
        content.push_str(&format!("mode = \"{}\"\n", self.preview.mode));
        content.push_str("\n[display]\n");
        content.push_str(&format!(
            "session_scope = \"{}\"\n",
            self.display.session_scope
        ));
        content.push_str("\n[telegram]\n");
        content.push_str(&format!("enabled = {}\n", self.telegram.enabled));
        content.push_str(&format!(
            "bot_token = \"{}\"\n",
            self.telegram.bot_token.replace('"', "\\\"")
        ));
        content.push_str(&format!(
            "chat_id = \"{}\"\n",
            self.telegram.chat_id.replace('"', "\\\"")
        ));
        content.push_str(&format!(
            "bot_username = \"{}\"\n",
            self.telegram.bot_username.replace('"', "\\\"")
        ));
        content.push_str("\n[agent_permissions]\n");
        content.push_str(&format!(
            "codex_auto_full_access = {}\n",
            self.agent_permissions.codex_auto_full_access
        ));
        content.push_str(&format!(
            "claude_auto_full_access = {}\n",
            self.agent_permissions.claude_auto_full_access
        ));
        content.push('\n');
        for agent in &self.agents {
            content.push_str("[[agents]]\n");
            content.push_str(&format!("name = \"{}\"\n", agent.name));
            content.push_str(&format!("cmd = \"{}\"\n", agent.cmd));
            if let Some(idx) = agent.active_provider {
                content.push_str(&format!("active_provider = {}\n", idx));
            }
            if !agent.default_model.is_empty() {
                content.push_str(&format!(
                    "default_model = \"{}\"\n",
                    agent.default_model.replace('"', "\\\"")
                ));
            }
            if !agent.small_model.is_empty() {
                content.push_str(&format!(
                    "small_model = \"{}\"\n",
                    agent.small_model.replace('"', "\\\"")
                ));
            }
            for prov in &agent.providers {
                content.push_str("\n[[agents.providers]]\n");
                content.push_str(&format!(
                    "label = \"{}\"\n",
                    prov.label.replace('"', "\\\"")
                ));
                content.push_str(&format!(
                    "base_url = \"{}\"\n",
                    prov.base_url.replace('"', "\\\"")
                ));
                content.push_str(&format!(
                    "api_key = \"{}\"\n",
                    prov.api_key.replace('"', "\\\"")
                ));
                content.push_str(&format!(
                    "wire_api = \"{}\"\n",
                    prov.codex_wire_api().replace('"', "\\\"")
                ));
                if !prov.provider_key.trim().is_empty() {
                    content.push_str(&format!(
                        "provider_key = \"{}\"\n",
                        prov.provider_key.replace('"', "\\\"")
                    ));
                }
                if !prov.npm_package.trim().is_empty() {
                    content.push_str(&format!(
                        "npm_package = \"{}\"\n",
                        prov.npm_package.replace('"', "\\\"")
                    ));
                }
                for model in &prov.models {
                    content.push_str("\n[[agents.providers.models]]\n");
                    content.push_str(&format!("id = \"{}\"\n", model.id.replace('"', "\\\"")));
                    content.push_str(&format!("name = \"{}\"\n", model.name.replace('"', "\\\"")));
                }
            }
            content.push('\n');
        }

        let _ = std::fs::write(&path, content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
        let _guard = crate::test_support::home_env_lock()
            .lock()
            .expect("lock theme tests");
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let home = std::env::temp_dir().join(format!("pad-theme-{name}-{stamp}"));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("create temp home");

        let prev_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &home);

        let result = f();

        if let Some(prev) = prev_home {
            std::env::set_var("HOME", prev);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = std::fs::remove_dir_all(&home);
        result
    }

    #[test]
    fn config_round_trips_opencode_provider_models() {
        with_temp_home("opencode-roundtrip", || {
            let mut config = Config::default();
            config.agent_permissions.codex_auto_full_access = false;
            let opencode = config
                .agents
                .iter_mut()
                .find(|agent| agent.name == "opencode")
                .expect("opencode agent");
            opencode.providers.push(ProviderConfig {
                label: "Relay".into(),
                base_url: "https://relay.example/v1".into(),
                api_key: "sk-test".into(),
                env_key: String::new(),
                wire_api: "responses".into(),
                provider_key: "relay".into(),
                npm_package: "@ai-sdk/openai-compatible".into(),
                models: vec![OpenCodeModelConfig {
                    id: "gpt-4o".into(),
                    name: "GPT-4o".into(),
                }],
                test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            });
            opencode.active_provider = Some(0);
            opencode.default_model = "relay/gpt-4o".into();
            opencode.small_model = "relay/gpt-4o-mini".into();
            config.save();

            let loaded = Config::load();
            assert!(!loaded.agent_permissions.codex_auto_full_access);
            assert!(loaded.agent_permissions.claude_auto_full_access);
            let opencode = loaded
                .agents
                .iter()
                .find(|agent| agent.name == "opencode")
                .expect("loaded opencode");
            assert_eq!(opencode.default_model, "relay/gpt-4o");
            assert_eq!(opencode.small_model, "");
            assert_eq!(opencode.providers.len(), 1);
            assert_eq!(opencode.providers[0].provider_key, "relay");
            assert_eq!(
                opencode.providers[0].npm_package,
                "@ai-sdk/openai-compatible"
            );
            assert_eq!(opencode.providers[0].models.len(), 1);
            assert_eq!(opencode.providers[0].models[0].id, "gpt-4o");
            assert_eq!(opencode.providers[0].models[0].name, "GPT-4o");
        });
    }

    #[test]
    fn config_defaults_agent_permissions_to_enabled() {
        with_temp_home("permissions-default", || {
            let config = Config::default();
            config.save();

            let loaded = Config::load();
            assert!(loaded.agent_permissions.codex_auto_full_access);
            assert!(loaded.agent_permissions.claude_auto_full_access);
        });
    }

    #[test]
    fn codex_base_url_candidates_try_root_and_v1_variants() {
        assert_eq!(
            codex_api_base_candidates("https://relay.example"),
            vec![
                "https://relay.example".to_string(),
                "https://relay.example/v1".to_string()
            ]
        );
        assert_eq!(
            codex_api_base_candidates("https://relay.example/v1"),
            vec![
                "https://relay.example/v1".to_string(),
                "https://relay.example".to_string()
            ]
        );
        assert_eq!(
            codex_api_base_candidates("https://relay.example/openai/v1"),
            vec!["https://relay.example/openai/v1".to_string()]
        );
    }

    #[test]
    fn codex_base_url_prefers_v1_for_root_inputs() {
        assert_eq!(
            codex_preferred_api_base_url("https://relay.example"),
            "https://relay.example/v1"
        );
        assert_eq!(
            codex_preferred_api_base_url("https://relay.example/"),
            "https://relay.example/v1"
        );
        assert_eq!(
            codex_preferred_api_base_url("https://relay.example/v1"),
            "https://relay.example/v1"
        );
        assert_eq!(
            codex_preferred_api_base_url("https://relay.example/openai/v1"),
            "https://relay.example/openai/v1"
        );
    }

    #[test]
    fn readability_boost_keeps_status_text_close_to_primary_fg() {
        let theme = Theme::by_name("catppuccin");
        assert_eq!(theme.status_fg, theme.fg);
    }

    #[test]
    fn readability_boost_lifts_comment_contrast() {
        let boosted = Theme::by_name("one-dark");
        assert_ne!(boosted.comment, Color::Rgb(92, 99, 112));
    }
}
