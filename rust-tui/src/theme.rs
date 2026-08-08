use ratatui::style::Color;
use reqwest::Url;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod agent;
mod color;
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
mod load;
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
mod save;
mod settings;

#[cfg(test)]
mod tests;

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
