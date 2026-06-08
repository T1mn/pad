use super::runtime::run_tmux_output;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub(super) fn probe_root_key_table(socket_name: &str, notes: &mut Vec<String>) -> bool {
    match run_tmux_output(socket_name, &["list-keys", "-T", "root"]) {
        Ok(_) => true,
        Err(err) => {
            notes.push(format!("root key-table probe failed: {err}"));
            false
        }
    }
}

pub(super) fn probe_control_mode_flags(socket_name: &str, notes: &mut Vec<String>) -> bool {
    let child = Command::new("tmux")
        .args([
            "-L",
            socket_name,
            "-C",
            "attach-session",
            "-t",
            "pad-probe",
            "-f",
            "read-only,ignore-size,no-output",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let Ok(mut child) = child else {
        notes.push("control-mode probe failed to spawn".to_string());
        return false;
    };

    thread::sleep(Duration::from_millis(150));
    match child.try_wait() {
        Ok(Some(_)) => {
            let output = child.wait_with_output();
            match output {
                Ok(output) => {
                    notes.push(format!(
                        "control-mode probe exited early: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
                Err(err) => {
                    notes.push(format!("control-mode probe wait failed: {err}"));
                }
            }
            false
        }
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            true
        }
        Err(err) => {
            notes.push(format!("control-mode probe status failed: {err}"));
            let _ = child.kill();
            let _ = child.wait();
            false
        }
    }
}

pub(super) fn probe_focus_events(socket_name: &str, notes: &mut Vec<String>) -> bool {
    if let Err(err) = run_tmux_output(socket_name, &["set", "-g", "focus-events", "on"]) {
        notes.push(format!("focus-events set probe failed: {err}"));
        return false;
    }

    match run_tmux_output(socket_name, &["show-options", "-gv", "focus-events"]) {
        Ok(stdout) => stdout.trim() == "on",
        Err(err) => {
            notes.push(format!("focus-events show probe failed: {err}"));
            false
        }
    }
}
