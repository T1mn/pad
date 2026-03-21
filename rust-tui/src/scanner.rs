use crate::model::{AgentPanel, AgentType, GitInfo};
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
        let current_cmd = parts[6];
        let working_dir = parts[7].to_string();

        let child_processes = get_child_processes(pane_pid);
        let all_processes = format!("{} {}", current_cmd, child_processes);

        let agent_type = AgentType::from_processes(&all_processes);

        if matches!(agent_type, AgentType::Unknown) {
            continue;
        }

        let is_active = check_active(&pane_id);
        let git_info = get_git_info(&working_dir);

        panels.push(AgentPanel {
            session,
            window,
            window_index,
            pane,
            pane_id,
            agent_type,
            working_dir,
            is_active,
            git_info,
            pid: Some(pane_pid.to_string()),
            start_time: Some(std::time::Instant::now()),
        });
    }

    panels.sort_by(|a, b| match (a.is_active, b.is_active) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
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
                if let Ok(cmd) = get_process_cmd(child_pid) {
                    processes.push(cmd);
                }
            }

            return processes.join(" ");
        }
    }

    String::new()
}

fn get_process_cmd(pid: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("ps")
        .args(["-p", pid, "-o", "comm=", "--no-headers"])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("Failed to get process cmd".into())
    }
}

fn check_active(pane_id: &str) -> bool {
    if let Ok(content) = capture_pane_content(pane_id, 20) {
        let content_lower = content.to_lowercase();
        let active_markers = [
            "thinking",
            "processing",
            "generating",
            "analyzing",
            "⋯",
            "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",
        ];

        for marker in &active_markers {
            if content_lower.contains(marker) {
                if content_lower.contains("claude")
                    || content_lower.contains("codex")
                    || content_lower.contains("kimi")
                    || content_lower.contains("gemini")
                    || content_lower.contains("opencode")
                    || content_lower.contains("aider")
                    || content_lower.contains("cursor")
                    || content_lower.contains("assistant")
                {
                    return true;
                }
            }
        }
    }

    false
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
