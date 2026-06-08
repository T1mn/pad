use crate::model::{AgentState, AgentType};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadActivityOverride {
    pub agent_type: AgentType,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub working_dir: String,
    pub state: AgentState,
    pub is_active: bool,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub updated_at: i64,
}

pub fn thread_sort_activity_keys(
    agent_type: &AgentType,
    session_id: Option<&str>,
    transcript_path: Option<&str>,
    working_dir: Option<&str>,
) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(session_id) = session_id.filter(|value| !value.trim().is_empty()) {
        keys.push(format!("{}:sid:{}", agent_type, session_id));
    }
    if let Some(transcript_path) = transcript_path.filter(|value| !value.trim().is_empty()) {
        keys.push(format!("{}:path:{}", agent_type, transcript_path));
    }
    if keys.is_empty() {
        if let Some(working_dir) = working_dir.filter(|value| !value.trim().is_empty()) {
            keys.push(format!("{}:cwd:{}", agent_type, working_dir));
        }
    }
    keys
}
