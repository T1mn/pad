use super::{
    AgentState, AgentStateSource, AgentType, GitInfo, SessionCacheState, SharedPreviewTurns,
};
use std::time::Instant;

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
    pub state_source: AgentStateSource,
    pub transcript_path: Option<String>,
    pub cached_preview_turns: SharedPreviewTurns,
    pub session_cache_state: Option<SessionCacheState>,
    pub git_info: Option<GitInfo>,
    pub pid: Option<String>,
    pub start_time: Option<Instant>,
    pub agent_session_id: Option<String>,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub has_unread_stop: bool,
}

impl AgentPanel {
    pub fn status_icon(&self, animation_frame: usize) -> &'static str {
        self.state.icon(animation_frame)
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

        if let Some((parent, leaf)) = trailing_path_segments(&path) {
            let short = format!("~/.../{parent}/{leaf}");
            if short.len() <= max_len {
                return short;
            }
        }

        // 安全截断：确保在字符边界处截断
        let start = path
            .char_indices()
            .rev()
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
                    &commit[..commit
                        .char_indices()
                        .nth(7)
                        .map(|(i, _)| i)
                        .unwrap_or(commit.len())],
                    git.changed_files
                )
            } else {
                let commit_short = &commit[..commit
                    .char_indices()
                    .nth(7)
                    .map(|(i, _)| i)
                    .unwrap_or(commit.len())];
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

fn trailing_path_segments(path: &str) -> Option<(&str, &str)> {
    let (prefix, leaf) = path.rsplit_once('/')?;
    let parent = prefix.rsplit('/').next()?;
    Some((parent, leaf))
}

fn get_process_uptime(pid: &str) -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-p", pid, "-o", "etimes="])
        .output()
        .ok()?;
    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().parse().ok()
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
