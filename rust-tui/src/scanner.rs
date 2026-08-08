mod detect {
    use super::process_snapshot::command_args_may_name_agent;
    use super::scan_caches::ScanCaches;
    use crate::model::AgentType;

    pub(super) fn detect_agent_type(
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

        if command_args_may_name_agent(&main_process) {
            let full_process = caches.cached_full_process_cmd(pane_pid).unwrap_or_default();
            agent_type = AgentType::from_processes(&full_process);
            if !matches!(agent_type, AgentType::Unknown) {
                return (agent_type, full_process, String::new());
            }
        }

        let child_processes = caches.cached_child_processes(pane_pid);
        agent_type = AgentType::from_processes(&child_processes);
        (agent_type, main_process, child_processes)
    }
}
mod git;
mod hydrate {
    use super::git::get_git_info_for_paths;
    use super::pane_capture::{capture_pane_content, capture_panes_content};
    use super::scan_caches::ScanCaches;
    use crate::model::{AgentPanel, AgentState};

    pub(super) fn hydrate_panel_runtime_state(panels: &mut [AgentPanel], caches: &mut ScanCaches) {
        let pane_ids = panels
            .iter()
            .map(|panel| panel.pane_id.clone())
            .collect::<Vec<_>>();
        let working_dirs = panels
            .iter()
            .map(|panel| panel.working_dir.clone())
            .collect::<Vec<_>>();
        let (captures, git_infos) = std::thread::scope(|scope| {
            let capture_handle =
                scope.spawn(|| capture_panes_content(&pane_ids, 20).unwrap_or_default());
            let git_handle = scope.spawn(|| get_git_info_for_paths(&working_dirs));
            (
                capture_handle.join().unwrap_or_default(),
                git_handle.join().unwrap_or_default(),
            )
        });

        for panel in panels {
            let state = captures
                .get(&panel.pane_id)
                .map(|content| crate::detector::detect_state(content))
                .or_else(|| {
                    capture_pane_content(&panel.pane_id, 20)
                        .ok()
                        .map(|content| crate::detector::detect_state(&content))
                })
                .unwrap_or(AgentState::Idle);
            panel.is_active = state == AgentState::Busy;
            panel.state = state;
            panel.git_info = git_infos
                .get(&panel.working_dir)
                .cloned()
                .unwrap_or_else(|| caches.cached_git_info(&panel.working_dir));
        }
    }
}
mod pane_capture;
mod panel {
    use super::tmux_panes::PaneLine;
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

    pub(super) fn panel_from_pane_line(
        pane_line: PaneLine<'_>,
        agent_type: AgentType,
    ) -> AgentPanel {
        AgentPanel {
            session: pane_line.session.to_string(),
            window: pane_line.window.to_string(),
            window_index: pane_line.window_index.to_string(),
            pane: pane_line.pane.to_string(),
            pane_id: pane_line.pane_id.to_string(),
            agent_type,
            working_dir: pane_line.working_dir.to_string(),
            is_active: false,
            state: AgentState::Idle,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            git_info: None,
            pid: Some(pane_line.pane_pid.to_string()),
            start_time: Some(std::time::Instant::now()),
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        }
    }
}
mod process_snapshot;
mod scan_caches {
    use super::git::get_git_info;
    use super::process_snapshot::ProcessSnapshot;
    use crate::model::GitInfo;
    use std::collections::HashMap;

    #[derive(Default)]
    pub(super) struct ScanCaches {
        pub(super) git_info: HashMap<String, Option<GitInfo>>,
        pub(super) processes: ProcessSnapshot,
    }

    impl ScanCaches {
        pub(super) fn with_pane_pids(pane_pids: Vec<String>) -> Self {
            Self {
                git_info: HashMap::new(),
                processes: ProcessSnapshot::for_root_pids(pane_pids),
            }
        }

        pub(super) fn cached_process_cmd(&mut self, pid: &str) -> Option<String> {
            self.processes.command(pid)
        }

        pub(super) fn cached_full_process_cmd(&mut self, pid: &str) -> Option<String> {
            self.processes.full_command(pid)
        }

        pub(super) fn cached_child_processes(&mut self, pid: &str) -> String {
            self.processes.child_processes(pid)
        }

        pub(super) fn cached_git_info(&mut self, working_dir: &str) -> Option<GitInfo> {
            if let Some(info) = self.git_info.get(working_dir) {
                return info.clone();
            }

            let info = get_git_info(working_dir);
            self.git_info.insert(working_dir.to_string(), info.clone());
            info
        }
    }
}
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
