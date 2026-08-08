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
