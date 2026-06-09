use crate::text_match::contains_ascii_ignore_case;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentType {
    Claude,
    Codex,
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
            AgentType::Kimi => "kimi",
            AgentType::Gemini => "gemini",
            AgentType::OpenCode => "opencode",
            AgentType::Aider => "aider",
            AgentType::Cursor => "cursor",
            AgentType::Unknown => "unknown",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            AgentType::Claude => "🟣C",
            AgentType::Codex => "🔵X",
            AgentType::Kimi => "🟢K",
            AgentType::Gemini => "🔷G",
            AgentType::OpenCode => "🟠O",
            AgentType::Aider => "🟡A",
            AgentType::Cursor => "🟤R",
            AgentType::Unknown => "⚪?",
        }
    }

    pub fn from_processes(processes: &str) -> Self {
        if contains_ascii_ignore_case(processes, "claude") {
            AgentType::Claude
        } else if contains_ascii_ignore_case(processes, "codex") {
            AgentType::Codex
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

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitInfo {
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub changed_files: usize,
}

/// Agent state detected from pane content
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentState {
    Idle,
    Busy,
    Waiting,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentStateSource {
    Scanner,
    Hook,
}

impl AgentState {
    pub fn icon(&self, animation_frame: usize) -> &'static str {
        match self {
            AgentState::Idle => " ",
            AgentState::Busy => match animation_frame % 10 {
                0 => "⠋",
                1 => "⠙",
                2 => "⠹",
                3 => "⠸",
                4 => "⠼",
                5 => "⠴",
                6 => "⠦",
                7 => "⠧",
                8 => "⠇",
                _ => "⠏",
            },
            AgentState::Waiting => "●",
        }
    }
}
