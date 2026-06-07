#[derive(Clone, Debug, PartialEq)]
pub struct DesiredAgentStyle {
    pub zoom: String,
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
pub struct SoundEventConfig {
    pub enabled: bool,
    pub preset: String,
}

impl SoundEventConfig {
    pub(super) fn new(enabled: bool, preset: &str) -> Self {
        Self {
            enabled,
            preset: preset.to_string(),
        }
    }

    pub(super) fn normalize_preset_for(event: crate::sound::SoundEvent, value: &str) -> String {
        crate::sound::normalize_preset_id_or_default(value, event.default_preset_id()).to_string()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoundConfig {
    pub enabled: bool,
    pub completion: SoundEventConfig,
    pub approval: SoundEventConfig,
    pub timeout: SoundEventConfig,
    pub failure: SoundEventConfig,
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            completion: SoundEventConfig::new(true, "glass"),
            approval: SoundEventConfig::new(false, "ping"),
            timeout: SoundEventConfig::new(false, "warning"),
            failure: SoundEventConfig::new(false, "alert"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodexConfig {
    pub fast_mode: bool,
    pub goals: bool,
    pub multi_agent: bool,
    pub web_search: String,
    pub status_line_model_with_reasoning: bool,
    pub status_line_fast_mode: bool,
    pub status_line_context_remaining: bool,
    pub status_line_current_dir: bool,
    pub jailbreak_prompt_file: bool,
    pub index_prompt_file: bool,
    pub title_summary: bool,
    pub show_qa_preview: bool,
}

impl CodexConfig {
    pub(super) fn normalized_web_search(value: &str) -> String {
        match value {
            "disabled" => "disabled".to_string(),
            "live" => "live".to_string(),
            "cached" => "cached".to_string(),
            _ => "default".to_string(),
        }
    }

    pub fn status_line_items(&self) -> Vec<&'static str> {
        let mut items = Vec::new();
        if self.status_line_model_with_reasoning {
            items.push("model-with-reasoning");
        }
        if self.status_line_fast_mode {
            items.push("fast-mode");
        }
        if self.status_line_context_remaining {
            items.push("context-remaining");
        }
        if self.status_line_current_dir {
            items.push("current-dir");
        }
        items
    }
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            fast_mode: false,
            goals: true,
            multi_agent: false,
            web_search: "default".to_string(),
            status_line_model_with_reasoning: false,
            status_line_fast_mode: false,
            status_line_context_remaining: false,
            status_line_current_dir: false,
            jailbreak_prompt_file: false,
            index_prompt_file: false,
            title_summary: false,
            show_qa_preview: false,
        }
    }
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
