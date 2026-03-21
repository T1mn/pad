use std::fmt;
use std::time::Instant;

#[derive(Clone, Debug)]
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
        match self {
            AgentType::Claude => write!(f, "claude"),
            AgentType::Codex => write!(f, "codex"),
            AgentType::Kimi => write!(f, "kimi"),
            AgentType::Gemini => write!(f, "gemini"),
            AgentType::OpenCode => write!(f, "opencode"),
            AgentType::Aider => write!(f, "aider"),
            AgentType::Cursor => write!(f, "cursor"),
            AgentType::Unknown => write!(f, "unknown"),
        }
    }
}

impl AgentType {
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
        let p = processes.to_lowercase();
        if p.contains("claude") {
            AgentType::Claude
        } else if p.contains("codex") {
            AgentType::Codex
        } else if p.contains("kimi") {
            AgentType::Kimi
        } else if p.contains("gemini") {
            AgentType::Gemini
        } else if p.contains("opencode") {
            AgentType::OpenCode
        } else if p.contains("aider") {
            AgentType::Aider
        } else if p.contains("cursor") {
            AgentType::Cursor
        } else {
            AgentType::Unknown
        }
    }
}

#[derive(Clone, Debug)]
pub struct GitInfo {
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub changed_files: usize,
}

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
    pub git_info: Option<GitInfo>,
    pub pid: Option<String>,
    pub start_time: Option<Instant>,
}

impl AgentPanel {
    #[allow(dead_code)]
    pub fn full_id(&self) -> String {
        format!("{}:{}.{}", self.session, self.window, self.pane)
    }

    pub fn status_icon(&self) -> &'static str {
        if self.is_active {
            "⚡"
        } else {
            "○"
        }
    }

    pub fn shortened_path(&self, max_len: usize) -> String {
        let path = &self.working_dir;
        let home = std::env::var("HOME").unwrap_or_default();
        let path = if path.starts_with(&home) {
            path.replacen(&home, "~", 1)
        } else {
            path.to_string()
        };

        if path.len() <= max_len {
            return path;
        }

        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            let short = format!("~/.../{}/{}", parts[parts.len() - 2], parts[parts.len() - 1]);
            if short.len() <= max_len {
                return short;
            }
        }

        // 安全截断：确保在字符边界处截断
        let start = path.char_indices().rev()
            .find(|(i, _)| path.len() - i <= max_len - 3)
            .map(|(i, _)| i)
            .unwrap_or(0);
        format!("...{}", &path[start..])
    }

    pub fn git_display(&self) -> String {
        if let Some(git) = &self.git_info {
            let branch = git.branch.as_deref().unwrap_or("?");
            let commit = git.commit.as_deref().unwrap_or("?");
            if git.changed_files > 0 {
                format!(
                    "{}@{}(+{})",
                    branch,
                    &commit[..commit.char_indices().nth(7).map(|(i, _)| i).unwrap_or(commit.len())],
                    git.changed_files
                )
            } else {
                let commit_short = &commit[..commit.char_indices().nth(7).map(|(i, _)| i).unwrap_or(commit.len())];
                format!("{}@{}", branch, commit_short)
            }
        } else {
            String::new()
        }
    }

    pub fn uptime_display(&self) -> String {
        if let Some(pid) = &self.pid {
            if let Some(secs) = get_process_uptime(pid) {
                return format_duration(secs);
            }
        }
        if let Some(start) = self.start_time {
            return format_duration(start.elapsed().as_secs());
        }
        "?".to_string()
    }
}

fn get_process_uptime(pid: &str) -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-p", pid, "-o", "etimes="])
        .output()
        .ok()?;
    if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .ok()
    } else {
        None
    }
}

fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}
