use std::fmt;

#[derive(Clone, Debug)]
pub enum CodeType {
    Claude,
    Codex,
    Kimi,
    Unknown,
}

impl fmt::Display for CodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodeType::Claude => write!(f, "claude"),
            CodeType::Codex => write!(f, "codex"),
            CodeType::Kimi => write!(f, "kimi"),
            CodeType::Unknown => write!(f, "unknown"),
        }
    }
}

impl CodeType {
    pub fn emoji(&self) -> &'static str {
        match self {
            CodeType::Claude => "🟣C",
            CodeType::Codex => "🔵X",
            CodeType::Kimi => "🟢K",
            CodeType::Unknown => "⚪?",
        }
    }

    pub fn from_processes(processes: &str) -> Self {
        let p = processes.to_lowercase();
        if p.contains("claude") {
            CodeType::Claude
        } else if p.contains("codex") {
            CodeType::Codex
        } else if p.contains("kimi") {
            CodeType::Kimi
        } else {
            CodeType::Unknown
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
pub struct CodePanel {
    pub session: String,
    pub window: String,
    pub window_index: String,
    pub pane: String,
    pub pane_id: String,
    pub code_type: CodeType,
    pub working_dir: String,
    pub is_active: bool,
    pub git_info: Option<GitInfo>,
}

impl CodePanel {
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

        // Try to keep last 2 components
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            let short = format!("~/.../{}/{}", parts[parts.len() - 2], parts[parts.len() - 1]);
            if short.len() <= max_len {
                return short;
            }
        }

        format!("...{}", &path[path.len().saturating_sub(max_len - 3)..])
    }

    pub fn git_display(&self) -> String {
        if let Some(git) = &self.git_info {
            let branch = git.branch.as_deref().unwrap_or("?");
            let commit = git.commit.as_deref().unwrap_or("?");
            if git.changed_files > 0 {
                format!("{}@{}(+{})", branch, &commit[..commit.len().min(7)], git.changed_files)
            } else {
                format!("{}@{}", branch, &commit[..commit.len().min(7)])
            }
        } else {
            String::new()
        }
    }
}
