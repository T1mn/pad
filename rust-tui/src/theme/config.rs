use super::*;

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
            desired_agent_style: DesiredAgentStyle::default(),
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
