#[derive(Clone, Debug, PartialEq)]
pub struct PreviewConfig {
    pub mode: String,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            mode: "session".to_string(),
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

/// Per-profile execution defaults owned by PAD.
///
/// This section is intentionally separate from `AgentPermissionsConfig`.
/// The latter contains the legacy Codex/Claude launch flags and must keep its
/// existing meaning.  A profile can therefore opt into PAD's execution policy
/// without changing (or rewriting) either provider's native configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileConfig {
    /// Human-readable profile name used by the desktop/sidebar profile picker.
    pub name: String,
    /// Canonical default mode: `guarded`, `workspace_full_access`, or
    /// `system_full_access`.
    pub default_permission_mode: String,
    /// Compatibility/UX switch for clients that expose a single Full Access
    /// toggle.  When enabled, the effective mode is system full access.
    pub full_access: bool,
    /// Allow the profile to continue without foreground confirmation prompts.
    pub unattended: bool,
}

impl ProfileConfig {
    pub const GUARDED: &'static str = "guarded";
    pub const WORKSPACE_FULL_ACCESS: &'static str = "workspace_full_access";
    pub const SYSTEM_FULL_ACCESS: &'static str = "system_full_access";

    pub fn normalized_permission_mode(value: &str) -> String {
        match value.trim().to_ascii_lowercase().as_str() {
            Self::WORKSPACE_FULL_ACCESS | "workspace" | "workspace-full-access" => {
                Self::WORKSPACE_FULL_ACCESS.to_string()
            }
            Self::SYSTEM_FULL_ACCESS
            | "system"
            | "system-full-access"
            | "full_access"
            | "full-access" => Self::SYSTEM_FULL_ACCESS.to_string(),
            _ => Self::GUARDED.to_string(),
        }
    }

    /// Returns the mode that should be consumed by a policy/runtime layer.
    ///
    /// `full_access` remains an explicit compatibility alias so an older
    /// profile file can be upgraded without silently disabling access.
    pub fn effective_permission_mode(&self) -> &str {
        if self.full_access {
            Self::SYSTEM_FULL_ACCESS
        } else {
            self.default_permission_mode.as_str()
        }
    }

    pub fn set_permission_mode(&mut self, mode: &str) {
        self.default_permission_mode = Self::normalized_permission_mode(mode);
        self.full_access = self.default_permission_mode == Self::SYSTEM_FULL_ACCESS;
    }

    pub fn set_full_access(&mut self, enabled: bool) {
        self.full_access = enabled;
        if enabled {
            self.default_permission_mode = Self::SYSTEM_FULL_ACCESS.to_string();
        } else if self.default_permission_mode == Self::SYSTEM_FULL_ACCESS {
            self.default_permission_mode = Self::GUARDED.to_string();
        }
    }
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            default_permission_mode: Self::GUARDED.to_string(),
            full_access: false,
            unattended: false,
        }
    }
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
    pub status_line_five_hour_limit: bool,
    pub status_line_weekly_limit: bool,
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
        if self.status_line_five_hour_limit {
            items.push("five-hour-limit");
        }
        if self.status_line_weekly_limit {
            items.push("weekly-limit");
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
            status_line_five_hour_limit: false,
            status_line_weekly_limit: false,
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
    pub agent_panel_width: Option<u16>,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            session_scope: "live".to_string(),
            agent_panel_width: None,
        }
    }
}
