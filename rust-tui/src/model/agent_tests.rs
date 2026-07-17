use super::AgentType;

#[test]
fn from_processes_detects_agent_case_insensitively() {
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

#[test]
fn from_processes_returns_unknown_without_agent_name() {
    assert_eq!(
        AgentType::from_processes("bash zsh tmux"),
        AgentType::Unknown
    );
}
