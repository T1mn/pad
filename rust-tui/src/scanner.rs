mod process_snapshot;

use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType, GitInfo};
use process_snapshot::ProcessSnapshot;
use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

pub fn scan_panels() -> Result<Vec<AgentPanel>, Box<dyn std::error::Error + Send + Sync>> {
    let scan_started_at = Instant::now();
    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-a",
            "-F",
            "#{session_name}|#{window_name}|#{window_index}|#{pane_index}|#{pane_id}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}",
        ])
        .output()?;

    if !output.status.success() {
        return Err("tmux list-panes failed".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let total_panes = stdout.lines().count();

    let mut panels = Vec::new();
    let mut caches = ScanCaches::default();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 8 {
            continue;
        }

        let session = parts[0].to_string();
        let window = parts[1].to_string();
        let window_index = parts[2].to_string();
        let pane = parts[3].to_string();
        let pane_id = parts[4].to_string();
        let pane_pid = parts[5].to_string();
        let current_cmd = parts[6];
        let working_dir = parts[7].to_string();

        let (agent_type, main_process, child_processes) =
            detect_agent_type(current_cmd, &pane_pid, &mut caches);

        if matches!(agent_type, AgentType::Unknown) {
            continue;
        }

        // Detect three-state from pane content
        let state = if let Ok(content) = capture_pane_content(&pane_id, 20) {
            crate::detector::detect_state(&content)
        } else {
            AgentState::Idle
        };
        let is_active = state == AgentState::Busy;
        let git_info = caches.cached_git_info(&working_dir);

        log_debug!(
            "scanner: agent pane={} session={} agent={:?} state={:?} dir={} main=[{}] children=[{}]",
            pane_id,
            session,
            agent_type,
            state,
            working_dir,
            main_process,
            child_processes
        );

        panels.push(AgentPanel {
            session,
            window,
            window_index,
            pane,
            pane_id,
            agent_type,
            working_dir,
            is_active,
            state,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            git_info,
            pid: Some(pane_pid),
            start_time: Some(std::time::Instant::now()),
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });
    }

    let elapsed = scan_started_at.elapsed();
    log_debug!(
        "scanner: completed panes={} agents={} elapsed_ms={}",
        total_panes,
        panels.len(),
        elapsed.as_millis()
    );

    // Sort: Waiting > Busy > Idle
    panels.sort_by(|a, b| {
        let state_ord = |s: &AgentState| -> u8 {
            match s {
                AgentState::Waiting => 0,
                AgentState::Busy => 1,
                AgentState::Idle => 2,
            }
        };
        state_ord(&a.state).cmp(&state_ord(&b.state))
    });

    Ok(panels)
}

#[derive(Default)]
struct ScanCaches {
    git_info: HashMap<String, Option<GitInfo>>,
    processes: ProcessSnapshot,
}

impl ScanCaches {
    fn cached_process_cmd(&mut self, pid: &str) -> Option<String> {
        self.processes.command(pid)
    }

    fn cached_child_processes(&mut self, pid: &str) -> String {
        self.processes.child_processes(pid)
    }

    fn cached_git_info(&mut self, working_dir: &str) -> Option<GitInfo> {
        if let Some(info) = self.git_info.get(working_dir) {
            return info.clone();
        }

        let info = get_git_info(working_dir);
        self.git_info.insert(working_dir.to_string(), info.clone());
        info
    }
}

fn detect_agent_type(
    current_cmd: &str,
    pane_pid: &str,
    caches: &mut ScanCaches,
) -> (AgentType, String, String) {
    let current_process = current_cmd.trim().to_string();
    let mut agent_type = AgentType::from_processes(&current_process);
    if !matches!(agent_type, AgentType::Unknown) {
        return (agent_type, current_process, String::new());
    }

    let main_process = caches.cached_process_cmd(pane_pid).unwrap_or_default();
    agent_type = AgentType::from_processes(&main_process);
    if !matches!(agent_type, AgentType::Unknown) {
        return (agent_type, main_process, String::new());
    }

    let child_processes = caches.cached_child_processes(pane_pid);
    agent_type = AgentType::from_processes(&child_processes);
    (agent_type, main_process, child_processes)
}

fn capture_pane_content(
    pane_id: &str,
    lines: usize,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("tmux")
        .args([
            "capture-pane",
            "-p",
            "-t",
            pane_id,
            "-S",
            &format!("-{}", lines),
        ])
        .output()?;

    if output.status.success() {
        Ok(strip_ansi(&String::from_utf8_lossy(&output.stdout)))
    } else {
        Err("Failed to capture pane".into())
    }
}

/// Strip ANSI escape sequences and control characters from captured pane content
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ESC [ ... final_byte (CSI sequences)
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc.is_ascii_alphabetic() || nc == 'm' || nc == '~' {
                        break;
                    }
                }
            } else {
                // Skip other ESC sequences (e.g. ESC ] for OSC)
                if let Some(&nc) = chars.peek() {
                    if nc == ']' {
                        // OSC: skip until ST (ESC \ or BEL)
                        chars.next();
                        while let Some(oc) = chars.next() {
                            if oc == '\x07' {
                                break;
                            }
                            if oc == '\x1b' && chars.peek() == Some(&'\\') {
                                chars.next();
                                break;
                            }
                        }
                    } else {
                        chars.next(); // skip single char after ESC
                    }
                }
            }
        } else if c == '\n' || c == '\t' || !c.is_control() {
            result.push(c);
        }
    }
    result
}

fn get_git_info(working_dir: &str) -> Option<GitInfo> {
    let output = Command::new("git")
        .args(["-C", working_dir, "rev-parse", "--git-dir"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = Command::new("git")
        .args(["-C", working_dir, "branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    let commit = Command::new("git")
        .args(["-C", working_dir, "rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    let changed_files = Command::new("git")
        .args(["-C", working_dir, "status", "--porcelain"])
        .output()
        .ok()
        .map(|o| {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout).lines().count()
            } else {
                0
            }
        })
        .unwrap_or(0);

    Some(GitInfo {
        branch,
        commit,
        changed_files,
    })
}

#[cfg(test)]
mod tests {
    use super::{detect_agent_type, ScanCaches};
    use crate::model::{AgentType, GitInfo};

    #[test]
    fn detect_agent_type_prefers_tmux_current_command() {
        let mut caches = ScanCaches::default();
        let (agent_type, main_process, child_processes) =
            detect_agent_type("codex", "123", &mut caches);

        assert_eq!(agent_type, AgentType::Codex);
        assert_eq!(main_process, "codex");
        assert!(child_processes.is_empty());
        assert!(!caches.processes.is_loaded());
    }

    #[test]
    fn cached_git_info_reuses_existing_result() {
        let mut caches = ScanCaches::default();
        caches.git_info.insert(
            "/tmp/project".to_string(),
            Some(GitInfo {
                branch: Some("main".to_string()),
                commit: Some("abc".to_string()),
                changed_files: 3,
            }),
        );

        let first = caches.cached_git_info("/tmp/project");
        let second = caches.cached_git_info("/tmp/project");

        assert_eq!(first, second);
        assert_eq!(caches.git_info.len(), 1);
    }
}
