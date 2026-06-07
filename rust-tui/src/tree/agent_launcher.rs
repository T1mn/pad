use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};
use std::path::PathBuf;

/// Agent launcher state
pub struct AgentLauncher {
    pub selected: usize,
    pub agents: Vec<(String, String)>, // (display_name, command)
    pub target_dir: PathBuf,
}

impl AgentLauncher {
    #[allow(dead_code)]
    pub fn new(target_dir: PathBuf) -> Self {
        Self::with_agents(target_dir, Vec::new())
    }

    pub fn with_agents(target_dir: PathBuf, agents: Vec<(String, String)>) -> Self {
        let agents = if agents.is_empty() {
            vec![
                ("claude-code".to_string(), "claude".to_string()),
                ("codex".to_string(), "codex".to_string()),
                ("kimi-cli".to_string(), "kimi-cli".to_string()),
                ("gemini-cli".to_string(), "gemini-cli".to_string()),
                ("opencode".to_string(), "opencode".to_string()),
                ("aider".to_string(), "aider".to_string()),
                ("cursor".to_string(), "cursor".to_string()),
            ]
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

    /// Render agent selector popup
    #[allow(dead_code)]
    pub fn render(&self, f: &mut Frame, area: Rect) {
        // Calculate popup area (centered)
        let popup_width = 40;
        let popup_height = 10;
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(
            area.x + popup_x,
            area.y + popup_y,
            popup_width,
            popup_height,
        );

        // Clear background
        f.render_widget(Clear, popup_area);

        // Create items
        let items: Vec<ListItem> = self
            .agents
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                let prefix = if i == self.selected { "❯ " } else { "  " };
                let content = format!("{}{}", prefix, name);
                ListItem::new(content)
            })
            .collect();

        let title = format!("Select Agent for {}", self.target_dir.display());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        // Use a dummy state since we're managing selection ourselves
        let mut state = ListState::default();
        state.select(Some(self.selected));
        f.render_stateful_widget(list, popup_area, &mut state);
    }

    /// Launch selected agent in given tmux session
    #[allow(dead_code)]
    pub fn launch(&self, session: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some((_, cmd)) = self.selected_agent() {
            let target_dir = self.target_dir.to_string_lossy();

            // Build tmux command: new-window -t session -c dir -n agent cmd
            let output = std::process::Command::new("tmux")
                .args([
                    "new-window",
                    "-t",
                    session,
                    "-c",
                    &target_dir,
                    "-n",
                    cmd,
                    cmd,
                ])
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to launch agent: {}", stderr).into());
            }
        }
        Ok(())
    }
}
