use super::append_spaced_command;
use std::process::Command;

pub(super) fn get_child_processes<F>(pid: &str, mut process_cmd_lookup: F) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    let output = Command::new("pgrep").args(["-P", pid]).output().ok();

    if let Some(output) = output {
        if output.status.success() {
            let child_pids = String::from_utf8_lossy(&output.stdout);
            let mut processes = String::new();

            for child_pid in child_pids.lines() {
                let child_pid = child_pid.trim();
                if child_pid.is_empty() {
                    continue;
                }
                if let Some(cmd) = process_cmd_lookup(child_pid) {
                    append_spaced_command(&mut processes, &cmd);
                }
            }

            return processes;
        }
    }

    String::new()
}

pub(super) fn get_process_cmd(
    pid: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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
