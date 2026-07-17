use std::path::PathBuf;

const DEFAULT_AGENTS: &[(&str, &str)] = &[
    ("claude-code", "claude"),
    ("codex", "codex"),
    ("grok-build", "grok"),
    ("kimi-cli", "kimi-cli"),
    ("gemini-cli", "gemini-cli"),
    ("opencode", "opencode"),
    ("aider", "aider"),
    ("cursor", "cursor"),
];

/// Agent launcher state.
pub struct AgentLauncher {
    pub selected: usize,
    pub agents: Vec<(String, String)>,
    pub target_dir: PathBuf,
}

impl AgentLauncher {
    pub fn with_agents(target_dir: PathBuf, agents: Vec<(String, String)>) -> Self {
        let agents = if agents.is_empty() {
            default_agents()
        } else {
            agents
        };
        Self {
            selected: 0,
            agents,
            target_dir,
        }
    }

    pub fn next(&mut self) {
        if self.selected < self.agents.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn selected_agent(&self) -> Option<&(String, String)> {
        self.agents.get(self.selected)
    }
}

fn default_agents() -> Vec<(String, String)> {
    DEFAULT_AGENTS
        .iter()
        .map(|(name, command)| ((*name).to_string(), (*command).to_string()))
        .collect()
}
