use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType, GitInfo};
use std::process::Command;

pub fn scan_panels() -> Result<Vec<AgentPanel>, Box<dyn std::error::Error + Send + Sync>> {
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
    log_debug!("scanner: list-panes 返回 {} 行", total_panes);

    let mut panels = Vec::new();

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
        let pane_pid = parts[5];
        let _current_cmd = parts[6];
        let working_dir = parts[7].to_string();

        let main_process = get_process_cmd(pane_pid).unwrap_or_default();
        let mut agent_type = AgentType::from_processes(&main_process);
        let child_processes = if matches!(agent_type, AgentType::Unknown) {
            get_child_processes(pane_pid)
        } else {
            String::new()
        };
        if matches!(agent_type, AgentType::Unknown) {
            agent_type = AgentType::from_processes(&child_processes);
        }

        log_debug!(
            "scanner: pane={} session={} main=[{}] children=[{}] -> agent={:?}",
            pane_id, session, main_process, child_processes, agent_type
        );

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
        let git_info = get_git_info(&working_dir);

        log_debug!(
            "scanner: 检测到智能体面板 pane={} agent={:?} state={:?} dir={}",
            pane_id, agent_type, state, working_dir
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
            git_info,
            pid: Some(pane_pid.to_string()),
            start_time: Some(std::time::Instant::now()),
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
        });
    }

    log_debug!("scanner: 共检测到 {} 个智能体面板", panels.len());

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

fn get_child_processes(pid: &str) -> String {
    let output = Command::new("pgrep").args(["-P", pid]).output().ok();

    if let Some(output) = output {
        if output.status.success() {
            let child_pids = String::from_utf8_lossy(&output.stdout);
            let mut processes = Vec::new();

            for child_pid in child_pids.lines() {
                let child_pid = child_pid.trim();
                if child_pid.is_empty() { continue; }
                if let Ok(cmd) = get_process_cmd(child_pid) {
                    log_debug!("scanner: child pid={} cmd={}", child_pid, cmd);
                    processes.push(cmd);
                }
            }

            return processes.join(" ");
        }
    }

    String::new()
}

fn get_process_cmd(pid: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Use 'args=' instead of 'comm=' to get full command path (avoids macOS 15-char truncation)
    let output = Command::new("ps")
        .args(["-p", pid, "-o", "args="])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("Failed to get process cmd".into())
    }
}

fn capture_pane_content(
    pane_id: &str,
    lines: usize,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-p", "-t", pane_id, "-S", &format!("-{}", lines)])
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
                            if oc == '\x07' { break; }
                            if oc == '\x1b' {
                                if chars.peek() == Some(&'\\') { chars.next(); break; }
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
