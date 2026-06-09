mod detect;
mod git;
mod hydrate;
mod pane_capture;
mod panel;
mod process_snapshot;
mod scan_caches;
mod tmux_panes;

use crate::model::{AgentPanel, AgentState, AgentType};
use detect::detect_agent_type;
use hydrate::hydrate_panel_runtime_state;
use panel::panel_from_pane_line;
use scan_caches::ScanCaches;
use std::process::Command;
use std::time::Instant;
use tmux_panes::{parse_pane_line, LIST_PANES_FORMAT};

pub use pane_capture::strip_ansi;

#[cfg(test)]
use git::get_git_info_for_paths;
#[cfg(test)]
use git::parse_git_status_porcelain_v2;
#[cfg(test)]
use pane_capture::{capture_pane_content, capture_panes_content};

pub fn scan_panels() -> Result<Vec<AgentPanel>, Box<dyn std::error::Error + Send + Sync>> {
    let scan_started_at = Instant::now();
    let output = Command::new("tmux")
        .args(["list-panes", "-a", "-F", LIST_PANES_FORMAT])
        .output()?;

    if !output.status.success() {
        return Err("tmux list-panes failed".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed_panes = parse_tmux_panes_output(&stdout);
    let mut caches = ScanCaches::with_pane_pids(parsed_panes.pane_pids.clone());

    let mut panels = Vec::new();
    let mut process_logs = Vec::new();
    for pane_line in parsed_panes.iter() {
        let (agent_type, main_process, child_processes) =
            detect_agent_type(pane_line.current_cmd, pane_line.pane_pid, &mut caches);

        if matches!(agent_type, AgentType::Unknown) {
            continue;
        }

        process_logs.push((panels.len(), main_process, child_processes));
        panels.push(panel_from_pane_line(pane_line, agent_type));
    }

    hydrate_panel_runtime_state(&mut panels, &mut caches);
    for (idx, main_process, child_processes) in process_logs {
        if let Some(panel) = panels.get(idx) {
            log_debug!(
                "scanner: agent pane={} session={} agent={:?} state={:?} dir={} main=[{}] children=[{}]",
                panel.pane_id,
                panel.session,
                panel.agent_type,
                panel.state,
                panel.working_dir,
                main_process,
                child_processes
            );
        }
    }

    let elapsed = scan_started_at.elapsed();
    log_debug!(
        "scanner: completed panes={} agents={} elapsed_ms={}",
        parsed_panes.total_panes,
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

struct ParsedPaneLines {
    total_panes: usize,
    lines: Vec<String>,
    pane_pids: Vec<String>,
}

impl ParsedPaneLines {
    fn iter(&self) -> impl Iterator<Item = tmux_panes::PaneLine<'_>> {
        self.lines.iter().filter_map(|line| parse_pane_line(line))
    }
}

fn parse_tmux_panes_output(stdout: &str) -> ParsedPaneLines {
    let mut parsed = ParsedPaneLines {
        total_panes: 0,
        lines: Vec::new(),
        pane_pids: Vec::new(),
    };
    for line in stdout.lines() {
        parsed.total_panes += 1;
        let Some(pane_line) = parse_pane_line(line) else {
            continue;
        };
        if !pane_line.pane_pid.trim().is_empty() {
            parsed.pane_pids.push(pane_line.pane_pid.to_string());
        }
        parsed.lines.push(line.to_string());
    }
    parsed
}

#[cfg(test)]
mod tests;
