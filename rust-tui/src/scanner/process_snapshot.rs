use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

#[derive(Default)]
pub(super) struct ProcessSnapshot {
    loaded: bool,
    commands: HashMap<String, String>,
    child_pids: HashMap<String, Vec<String>>,
}

impl ProcessSnapshot {
    pub(super) fn command(&mut self, pid: &str) -> Option<String> {
        self.ensure_loaded();
        if let Some(cmd) = self.commands.get(pid) {
            return Some(cmd.clone());
        }

        let cmd = get_process_cmd(pid).ok()?;
        self.commands.insert(pid.to_string(), cmd.clone());
        Some(cmd)
    }

    pub(super) fn child_processes(&mut self, pid: &str) -> String {
        self.ensure_loaded();
        if let Some(child_pids) = self.child_pids.get(pid) {
            return child_pids
                .iter()
                .filter_map(|child_pid| self.commands.get(child_pid))
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
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
        match load_process_snapshot() {
            Some((commands, child_pids)) => {
                let process_count = commands.len();
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
}

type ProcessMaps = (HashMap<String, String>, HashMap<String, Vec<String>>);

fn load_process_snapshot() -> Option<ProcessMaps> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,args="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    Some(parse_process_snapshot(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_process_snapshot(stdout: &str) -> ProcessMaps {
    let mut commands = HashMap::new();
    let mut child_pids: HashMap<String, Vec<String>> = HashMap::new();

    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(pid) = parts.next() else {
            continue;
        };
        let Some(ppid) = parts.next() else {
            continue;
        };
        let args = parts.collect::<Vec<_>>().join(" ");
        if args.is_empty() {
            continue;
        }

        commands.insert(pid.to_string(), args);
        child_pids
            .entry(ppid.to_string())
            .or_default()
            .push(pid.to_string());
    }

    (commands, child_pids)
}

fn get_child_processes<F>(pid: &str, mut process_cmd_lookup: F) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    let output = Command::new("pgrep").args(["-P", pid]).output().ok();

    if let Some(output) = output {
        if output.status.success() {
            let child_pids = String::from_utf8_lossy(&output.stdout);
            let mut processes = Vec::new();

            for child_pid in child_pids.lines() {
                let child_pid = child_pid.trim();
                if child_pid.is_empty() {
                    continue;
                }
                if let Some(cmd) = process_cmd_lookup(child_pid) {
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

#[cfg(test)]
mod tests {
    use super::parse_process_snapshot;

    #[test]
    fn process_snapshot_parses_pid_ppid_and_args() {
        let (commands, children) = parse_process_snapshot(
            "  10     1 /bin/zsh -l\n  11    10 /opt/homebrew/bin/codex --resume abc\n",
        );

        assert_eq!(commands.get("10").map(String::as_str), Some("/bin/zsh -l"));
        assert_eq!(
            commands.get("11").map(String::as_str),
            Some("/opt/homebrew/bin/codex --resume abc")
        );
        assert_eq!(children.get("10"), Some(&vec!["11".to_string()]));
    }
}
