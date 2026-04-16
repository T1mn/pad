use ratatui::style::Color;
use reqwest::Url;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod agent;
mod color;
mod config;
mod load;
mod palette_core;
mod palette_dark;
mod palette_light;
mod provider;
mod save;
mod settings;

#[cfg(test)]
mod tests;

pub use agent::{AgentConfig, OpenCodeModelConfig};
pub use config::Config;
pub use palette_core::Theme;
pub use provider::{normalize_provider_key, ProviderConfig};
pub use settings::{
    AgentPermissionsConfig, CodexConfig, DesiredAgentStyle, DisplayConfig, PreviewConfig,
    SoundConfig, SoundEventConfig, TelegramConfig,
};

pub(crate) use provider::codex_api_base_candidates;
