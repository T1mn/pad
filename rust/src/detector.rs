use regex::Regex;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::tmux::{capture_pane, get_all_child_processes, list_panes, PaneInfo};

#[derive(Debug, Clone)]
pub struct CodePanel {
    pub session: String,
    pub window: String,
    pub pane: String,
    pub pane_id: String,
    pub code_type: String,
    pub working_dir: String,
    pub is_active: bool,
    pub last_activity: f64,
}

const CODE_PATTERNS: &[(&str, &[&str])] = &[
    ("claude", &["claude"]),
    ("codex", &["codex"]),
    ("kimi", &["Kimi", "kimi", "Kimi Code"]),
];

const ACTIVE_MARKERS: &[&str] = &[
    "thinking",
    "loading",
    "processing",
    "generating",
    "running",
    "executing",
    "⋯",
    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",
];

pub async fn scan_code_panels() -> Vec<CodePanel> {
    let panes = list_panes().await;
    let mut code_panels = Vec::new();
    
    // Process panes concurrently
    let tasks: Vec<_> = panes
        .into_iter()
        .map(|pane| tokio::spawn(process_pane(pane)))
        .collect();
    
    for task in tasks {
        if let Ok(Some(panel)) = task.await {
            code_panels.push(panel);
        }
    }
    
    // Sort by last_activity (descending)
    code_panels.sort_by(|a, b| b.last_activity.partial_cmp(&a.last_activity).unwrap());
    
    code_panels
}

async fn process_pane(pane: PaneInfo) -> Option<CodePanel> {
    let pid = pane.pane_pid;
    if pid == 0 {
        return None;
    }
    
    // Get child processes
    let child_processes = get_all_child_processes(pid).await;
    let current_cmd = &pane.pane_current_command;
    
    // Detect code type
    let code_type = detect_code_type(current_cmd, &child_processes);
    if code_type.is_none() {
        return None;
    }
    
    let code_type = code_type.unwrap();
    
    // Capture pane content
    let content = capture_pane(&pane.pane_id).await;
    
    // Check if active
    let is_active = is_panel_active(&content);
    let last_activity = if is_active {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    } else {
        0.0
    };
    
    Some(CodePanel {
        session: pane.session_name,
        window: pane.window_name,
        pane: pane.pane_index.clone(),
        pane_id: pane.pane_id,
        code_type,
        working_dir: pane.pane_current_path,
        is_active,
        last_activity,
    })
}

fn detect_code_type(current_cmd: &str, child_processes: &[String]) -> Option<String> {
    let all_processes: HashSet<String> = std::iter::once(current_cmd.to_lowercase())
        .chain(child_processes.iter().map(|s| s.to_lowercase()))
        .collect();
    
    let process_str = all_processes.iter().cloned().collect::<Vec<_>>().join(" ");
    
    for (code_type, patterns) in CODE_PATTERNS {
        for pattern in *patterns {
            if process_str.contains(&pattern.to_lowercase()) {
                return Some(code_type.to_string());
            }
        }
    }
    
    None
}

fn is_panel_active(content: &str) -> bool {
    let content_lower = content.to_lowercase();
    ACTIVE_MARKERS.iter().any(|marker| content_lower.contains(marker))
}

pub fn extract_content_summary(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let non_empty: Vec<&str> = lines.iter().filter(|l| !l.trim().is_empty()).cloned().collect();
    
    let summary_lines: Vec<&str> = non_empty.iter().rev().take(max_lines).cloned().collect();
    let summary = summary_lines.join(" ");
    
    if summary.len() > 100 {
        format!("{}...", &summary[..97])
    } else {
        summary
    }
}
