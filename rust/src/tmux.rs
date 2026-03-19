use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub session_name: String,
    pub window_name: String,
    pub pane_index: String,
    pub pane_id: String,
    pub pane_pid: i32,
    pub pane_current_command: String,
    pub pane_current_path: String,
    pub pane_active: bool,
}

pub async fn list_panes() -> Vec<PaneInfo> {
    let format = "#{session_name}|#{window_name}|#{pane_index}|#{pane_id}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}|#{pane_active}";
    
    let output = Command::new("tmux")
        .args(&["list-panes", "-a", "-F", format])
        .output()
        .await
        .expect("Failed to execute tmux list-panes");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut panes = Vec::new();
    
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 8 {
            panes.push(PaneInfo {
                session_name: parts[0].to_string(),
                window_name: parts[1].to_string(),
                pane_index: parts[2].to_string(),
                pane_id: parts[3].to_string(),
                pane_pid: parts[4].parse().unwrap_or(0),
                pane_current_command: parts[5].to_string(),
                pane_current_path: parts[6].to_string(),
                pane_active: parts[7] == "1",
            });
        }
    }
    
    panes
}

pub async fn capture_pane(pane_id: &str) -> String {
    let output = Command::new("tmux")
        .args(&["capture-pane", "-p", "-t", pane_id, "-S", "-100"])
        .output()
        .await
        .expect("Failed to execute tmux capture-pane");
    
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub async fn get_all_child_processes(pid: i32) -> Vec<String> {
    let mut commands = Vec::new();
    let mut stack = vec![pid];
    
    while let Some(current_pid) = stack.pop() {
        let output = Command::new("ps")
            .args(&["--ppid", &current_pid.to_string(), "-o", "pid=,comm="])
            .output()
            .await;
        
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(child_pid) = parts[0].parse::<i32>() {
                        commands.push(parts[1].to_string());
                        stack.push(child_pid);
                    }
                }
            }
        }
    }
    
    commands
}

pub async fn get_pane_history_size(pane_id: &str) -> i32 {
    let output = Command::new("tmux")
        .args(&["list-panes", "-t", pane_id, "-F", "#{history_size}"])
        .output()
        .await;
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.trim().parse().unwrap_or(0)
    } else {
        0
    }
}
