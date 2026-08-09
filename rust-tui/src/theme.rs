use ratatui::style::Color;
use reqwest::Url;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod agent {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct OpenCodeModelConfig {
        pub id: String,
        pub name: String,
    }

    #[derive(Clone, Debug)]
    pub struct AgentConfig {
        pub name: String,
        pub cmd: String,
        pub providers: Vec<ProviderConfig>,
        pub active_provider: Option<usize>,
        pub default_model: String,
        pub small_model: String,
    }

    impl AgentConfig {
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
}
mod color {
    use super::*;

    pub(super) fn readable_text_color(primary: Color, current: Color, mix: f32) -> Color {
        blend_theme_color(primary, current, mix)
    }

    pub(super) fn readable_surface_color(primary: Color, current: Color, mix: f32) -> Color {
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
}
mod config {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Config {
        pub theme: String,
        pub auto_refresh: bool,
        pub refresh_interval: u64,
        pub agents: Vec<AgentConfig>,
        pub language: String,
        pub preview: PreviewConfig,
        pub display: DisplayConfig,
        pub sound: SoundConfig,
        pub telegram: TelegramConfig,
        pub codex: CodexConfig,
        pub agent_permissions: AgentPermissionsConfig,
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                theme: "default".to_string(),
                auto_refresh: true,
                refresh_interval: 10,
                agents: default_agents(),
                language: "en".to_string(),
                preview: PreviewConfig::default(),
                display: DisplayConfig::default(),
                sound: SoundConfig::default(),
                telegram: TelegramConfig::default(),
                codex: CodexConfig::default(),
                agent_permissions: AgentPermissionsConfig::default(),
            }
        }
    }

    impl Config {
        pub fn config_path() -> PathBuf {
            crate::paths::config_path()
        }

        pub fn resolved_config_path() -> Option<PathBuf> {
            let path = Self::config_path();
            let legacy_path = crate::paths::legacy_config_path();
            if path.exists() {
                Some(path)
            } else if legacy_path.exists() {
                Some(legacy_path)
            } else {
                None
            }
        }
    }

    fn default_agents() -> Vec<AgentConfig> {
        vec![
            default_agent("claude"),
            default_agent("codex"),
            default_agent("deepseek"),
            default_agent("gemini"),
            default_agent("opencode"),
        ]
    }

    pub(super) fn default_agent(name: &str) -> AgentConfig {
        AgentConfig {
            name: name.into(),
            cmd: name.into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
        }
    }
}
pub(crate) mod load;
mod palette_core {
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
            self.highlight_bg =
                super::color::readable_surface_color(self.fg, self.highlight_bg, 0.12);
            self
        }
    }
}
mod palette_dark;
mod palette_light;
mod provider;
pub(crate) mod save;
mod settings;

#[cfg(test)]
pub(crate) mod tests;

pub use agent::{AgentConfig, OpenCodeModelConfig};
pub use config::Config;
pub use load::ConfigRecovery;
pub use palette_core::Theme;
pub use provider::{normalize_provider_key, ProviderConfig};
pub use settings::{
    AgentPermissionsConfig, CodexConfig, DisplayConfig, PreviewConfig, SoundConfig,
    SoundEventConfig, TelegramConfig,
};

pub(crate) use provider::codex_api_base_candidates;
