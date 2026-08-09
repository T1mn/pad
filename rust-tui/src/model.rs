mod agent {
    use crate::text_match::contains_ascii_ignore_case;
    use std::fmt;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum AgentType {
        Claude,
        Codex,
        Grok,
        Kimi,
        Gemini,
        OpenCode,
        Aider,
        Cursor,
        Unknown,
    }

    impl fmt::Display for AgentType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.as_str())
        }
    }

    impl AgentType {
        pub fn as_str(&self) -> &'static str {
            match self {
                AgentType::Claude => "claude",
                AgentType::Codex => "codex",
                AgentType::Grok => "grok",
                AgentType::Kimi => "kimi",
                AgentType::Gemini => "gemini",
                AgentType::OpenCode => "opencode",
                AgentType::Aider => "aider",
                AgentType::Cursor => "cursor",
                AgentType::Unknown => "unknown",
            }
        }

        pub fn from_processes(processes: &str) -> Self {
            if contains_ascii_ignore_case(processes, "claude") {
                AgentType::Claude
            } else if contains_ascii_ignore_case(processes, "codex") {
                AgentType::Codex
            } else if contains_ascii_ignore_case(processes, "grok") {
                AgentType::Grok
            } else if contains_ascii_ignore_case(processes, "kimi") {
                AgentType::Kimi
            } else if contains_ascii_ignore_case(processes, "gemini") {
                AgentType::Gemini
            } else if contains_ascii_ignore_case(processes, "opencode") {
                AgentType::OpenCode
            } else if contains_ascii_ignore_case(processes, "aider") {
                AgentType::Aider
            } else if contains_ascii_ignore_case(processes, "cursor") {
                AgentType::Cursor
            } else {
                AgentType::Unknown
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub enum AgentState {
        Idle,
        Busy,
        Waiting,
    }
}
mod panel {
    use super::{AgentState, AgentType, SessionCacheState, SharedPreviewTurns};

    #[derive(Clone, Debug)]
    pub struct AgentPanel {
        pub session: String,
        pub window: String,
        pub window_index: String,
        pub pane: String,
        pub pane_id: String,
        pub agent_type: AgentType,
        pub working_dir: String,
        pub is_active: bool,
        pub state: AgentState,
        pub transcript_path: Option<String>,
        pub cached_preview_turns: SharedPreviewTurns,
        pub session_cache_state: Option<SessionCacheState>,
        pub agent_session_id: Option<String>,
        pub last_user_prompt: Option<String>,
        pub last_assistant_message: Option<String>,
        pub has_unread_stop: bool,
    }
}
mod preview {
    use serde::{Deserialize, Serialize};
    use std::ops::Deref;
    use std::sync::Arc;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PreviewSource {
        Plain,
        Session,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PreviewSessionOrigin {
        Pane,
        App,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PreviewView {
        Plain,
        SessionList,
        SessionDetail,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SessionCacheState {
        Cached,
        Confirmed,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct PreviewTurn {
        pub question: String,
        pub answer: Option<String>,
    }

    #[derive(Clone, Debug)]
    pub struct SharedPreviewTurns(Arc<[PreviewTurn]>);

    impl SharedPreviewTurns {
        pub fn to_vec(&self) -> Vec<PreviewTurn> {
            self.0.as_ref().to_vec()
        }

        pub fn shares_allocation_with(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.0, &other.0)
        }
    }

    impl Default for SharedPreviewTurns {
        fn default() -> Self {
            Self(Arc::from([]))
        }
    }

    impl From<Vec<PreviewTurn>> for SharedPreviewTurns {
        fn from(turns: Vec<PreviewTurn>) -> Self {
            Self(turns.into())
        }
    }

    impl Deref for SharedPreviewTurns {
        type Target = [PreviewTurn];

        fn deref(&self) -> &Self::Target {
            self.0.as_ref()
        }
    }

    impl AsRef<[PreviewTurn]> for SharedPreviewTurns {
        fn as_ref(&self) -> &[PreviewTurn] {
            self.0.as_ref()
        }
    }

    impl PartialEq for SharedPreviewTurns {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.0, &other.0) || self.0.as_ref() == other.0.as_ref()
        }
    }

    impl Eq for SharedPreviewTurns {}
}

pub use agent::{AgentState, AgentType};
pub use panel::AgentPanel;
pub use preview::{
    PreviewSessionOrigin, PreviewSource, PreviewTurn, PreviewView, SessionCacheState,
    SharedPreviewTurns,
};

#[cfg(test)]
pub(crate) mod tests {
    pub(crate) mod agent {
        use super::super::agent::AgentType;

        pub(crate) fn from_processes_detects_agent_case_insensitively() {
            assert_eq!(
                AgentType::from_processes("/usr/bin/CODEX"),
                AgentType::Codex
            );
            assert_eq!(
                AgentType::from_processes("node OpenCode"),
                AgentType::OpenCode
            );
            assert_eq!(
                AgentType::from_processes("/Users/tim/.grok/downloads/grok-0.2.102-macos-aarch64"),
                AgentType::Grok
            );
        }

        pub(crate) fn from_processes_returns_unknown_without_agent_name() {
            assert_eq!(
                AgentType::from_processes("bash zsh terminal"),
                AgentType::Unknown
            );
        }
    }

    pub(crate) mod preview {
        use super::super::preview::{PreviewTurn, SharedPreviewTurns};

        pub(crate) fn shared_preview_turns_clone_reuses_allocation() {
            let turns = SharedPreviewTurns::from(vec![PreviewTurn {
                question: "hello".into(),
                answer: Some("world".into()),
            }]);
            let cloned = turns.clone();

            assert!(turns.shares_allocation_with(&cloned));
            assert_eq!(cloned[0].question, "hello");
            assert_eq!(cloned[0].answer.as_deref(), Some("world"));
        }

        pub(crate) fn shared_preview_turns_equality_uses_same_allocation() {
            let turns = SharedPreviewTurns::from(vec![PreviewTurn {
                question: "hello".into(),
                answer: Some("world".into()),
            }]);
            let cloned = turns.clone();

            assert_eq!(turns, cloned);
        }
    }
}
