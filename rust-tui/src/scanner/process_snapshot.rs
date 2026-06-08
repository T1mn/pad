mod classify;
mod fallback;
mod loader;

pub(super) use classify::command_args_may_name_agent;
use classify::command_may_hide_agent;
use fallback::{get_child_processes, get_process_cmd};
use loader::{load_lightweight_process_snapshot, load_process_snapshot};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Default)]
pub(super) struct ProcessSnapshot {
    loaded: bool,
    snapshot_available: bool,
    root_pids: Vec<String>,
    commands: HashMap<String, String>,
    child_pids: HashMap<String, Vec<String>>,
    full_commands: HashMap<String, String>,
}

impl ProcessSnapshot {
    pub(super) fn for_root_pids(root_pids: Vec<String>) -> Self {
        Self {
            root_pids,
            ..Self::default()
        }
    }

    pub(super) fn command(&mut self, pid: &str) -> Option<String> {
        self.ensure_loaded();
        if let Some(cmd) = self.commands.get(pid) {
            return Some(cmd.clone());
        }

        self.full_command(pid)
    }

    pub(super) fn full_command(&mut self, pid: &str) -> Option<String> {
        if let Some(cmd) = self.full_commands.get(pid) {
            return Some(cmd.clone());
        }

        let cmd = get_process_cmd(pid).ok()?;
        if cmd.is_empty() {
            return None;
        }
        self.full_commands.insert(pid.to_string(), cmd.clone());
        self.commands
            .entry(pid.to_string())
            .or_insert_with(|| cmd.clone());
        Some(cmd)
    }

    pub(super) fn child_processes(&mut self, pid: &str) -> String {
        self.ensure_loaded();
        if let Some(child_pids) = self.child_pids.get(pid) {
            let child_pids = child_pids.clone();
            return child_pids
                .iter()
                .filter_map(|child_pid| self.command_with_expanded_args(child_pid))
                .collect::<Vec<_>>()
                .join(" ");
        }

        if self.snapshot_available {
            return String::new();
        }

        get_child_processes(pid, |child_pid| self.command(child_pid))
    }

    #[cfg(test)]
    pub(super) fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let started_at = Instant::now();
        let snapshot = load_lightweight_process_snapshot(&self.root_pids)
            .or_else(|| load_process_snapshot(&self.root_pids));
        match snapshot {
            Some((commands, child_pids)) => {
                let process_count = commands.len();
                self.snapshot_available = true;
                self.commands = commands;
                self.child_pids = child_pids;
                let elapsed = started_at.elapsed();
                if elapsed >= std::time::Duration::from_millis(20) {
                    crate::log_debug!(
                        "scanner.process_snapshot: loaded processes={} elapsed_ms={}",
                        process_count,
                        elapsed.as_millis()
                    );
                }
            }
            None => {
                crate::log_debug!(
                    "scanner.process_snapshot: load failed, falling back to per-pid ps"
                );
            }
        }
    }

    fn command_with_expanded_args(&mut self, pid: &str) -> Option<String> {
        let command = self.commands.get(pid)?.clone();
        if command_may_hide_agent(&command) {
            if let Some(full_command) = self.full_command(pid) {
                if full_command != command {
                    return Some(format!("{command} {full_command}"));
                }
            }
        }
        Some(command)
    }
}
