use super::activity::thread_sort_activity_keys;
use crate::model::{
    AgentState, AgentType, GitInfo, PreviewSessionOrigin, SessionCacheState, SharedPreviewTurns,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarThread {
    pub key: String,
    pub folder_key: String,
    pub working_dir: String,
    pub folder_label: String,
    pub agent_type: AgentType,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub session_provider_name: Option<String>,
    pub title: String,
    pub upstream_title: Option<String>,
    pub generated_title: Option<String>,
    pub subtitle: Option<String>,
    pub title_override: Option<String>,
    pub note: Option<String>,
    pub share_url: Option<String>,
    pub cost: Option<String>,
    pub token_summary: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub updated_at: i64,
    pub sort_updated_at: i64,
    pub live_pane_id: Option<String>,
    pub live_location: Option<String>,
    pub pid: Option<String>,
    pub git_info: Option<GitInfo>,
    pub state: AgentState,
    pub is_active: bool,
    pub cached_preview_turns: SharedPreviewTurns,
    pub session_cache_state: Option<SessionCacheState>,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub has_unread_stop: bool,
    pub archived: bool,
    pub deleted: bool,
}

impl SidebarThread {
    pub fn sort_timestamp(&self) -> i64 {
        self.sort_updated_at
    }

    pub fn preview_origin(&self) -> Option<PreviewSessionOrigin> {
        if self.agent_type == AgentType::Codex {
            return Some(if self.live_pane_id.is_some() {
                PreviewSessionOrigin::Pane
            } else {
                PreviewSessionOrigin::App
            });
        }

        None
    }

    pub fn is_live(&self) -> bool {
        self.live_pane_id.is_some()
    }

    pub fn sort_activity_keys(&self) -> Vec<String> {
        thread_sort_activity_keys(
            &self.agent_type,
            self.session_id.as_deref(),
            self.transcript_path.as_deref(),
            Some(self.working_dir.as_str()),
        )
    }
}
