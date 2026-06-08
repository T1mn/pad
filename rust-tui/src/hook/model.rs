use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookTmuxInfo {
    pub pane_id: Option<String>,
    pub session_name: Option<String>,
    pub window_index: Option<String>,
    pub pane_index: Option<String>,
    pub pane_current_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event: String,
    #[serde(default)]
    pub turn_id: Option<String>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub cwd: Option<String>,
    pub prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub timestamp: Option<String>,
    pub tmux: HookTmuxInfo,
}
