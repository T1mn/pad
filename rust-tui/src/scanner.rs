use crate::model::{CodePanel, CodeType, GitInfo};
use std::process::Command;

pub fn scan_panels() -> Result<Vec<CodePanel>, Box<dyn std::error::Error>> {
    let output = Command::new("tmux")
        .args(&[
            "list-panes",
            "-a",
            "-F",
            "#{session_name}|#{window_name}|#{pane_index}|#{pane_id}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}",
        ])
        .output()?;

    if !output.status.success() {
        return Err("tmux list-panes failed".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut panels = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 7 {
            continue;
        }

        let session = parts[0].to_string();
        let window = parts[1].to_string();
        let pane = parts[2].to_string();
        let pane_id = parts[3].to_string();
        let pane_pid = parts[4];
        let current_cmd = parts[5];
        let working_dir = parts[6].to_string();

        // Get child processes
        let child_processes = get_child_processes(pane_pid);
        let all_processes = format!("{} {}", current_cmd, child_processes);

        // Detect code type
        let code_type = CodeType::from_processes(&all_processes);

        // Skip if not a code panel
        if matches!(code_type, CodeType::Unknown) {
            continue;
        }

        // Check if active
        let is_active = check_active(&pane_id);

        // Get git info
        let git_info = get_git_info(&working_dir);

        panels.push(CodePanel {
            session,
            window,
            pane,
            pane_id,
            code_type,
            working_dir,
            is_active,
            git_info,
        });
    }

    // Sort by activity (active first, then by last activity time if available)
    panels.sort_by(|a, b| {
        match (a.is_active, b.is_active) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    });

    Ok(panels)
}

fn get_child_processes(pid: &str) -> String {
    let output = Command::new("pgrep")
        .args(&["-P", pid])
        .output()
        .ok();

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

fn get_process_cmd(pid: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("ps")
        .args(&["-p", pid, "-o", "comm=", "--no-headers"])
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("Failed to get process cmd".into())
    }
}

fn check_active(pane_id: &str) -> bool {
    // Capture recent content and check for active markers
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
        
        // Check for AI-specific markers
        for marker in &active_markers {
            if content_lower.contains(marker) {
                // Also check for AI context
                if content_lower.contains("claude") 
                    || content_lower.contains("codex")
                    || content_lower.contains("kimi")
                    || content_lower.contains("assistant") {
                    return true;
                }
            }
        }
    }
    
    false
}

fn capture_pane_content(pane_id: &str, lines: usize) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("tmux")
        .args(&["capture-pane", "-p", "-t", pane_id, "-S", &format!("-{}", lines)])
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("Failed to capture pane".into())
    }
}

fn get_git_info(working_dir: &str) -> Option<GitInfo> {
    // Check if git repo
    let output = Command::new("git")
        .args(&["-C", working_dir, "rev-parse", "--git-dir"])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }

    // Get branch
    let branch = Command::new("git")
        .args(&["-C", working_dir, "branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    // Get commit
    let commit = Command::new("git")
        .args(&["-C", working_dir, "rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    // Get changed files count
    let changed_files = Command::new("git")
        .args(&["-C", working_dir, "status", "--porcelain"])
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
