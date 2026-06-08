use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::tmux_capabilities) fn start_probe_server(socket_name: &str) -> Result<(), String> {
    let output = Command::new("tmux")
        .args([
            "-L",
            socket_name,
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-s",
            "pad-probe",
            "-x",
            "120",
            "-y",
            "40",
            "sh",
        ])
        .output()
        .map_err(|err| err.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub(in crate::tmux_capabilities) fn stop_probe_server(socket_name: &str) -> Result<(), String> {
    let output = Command::new("tmux")
        .args(["-L", socket_name, "kill-server"])
        .output()
        .map_err(|err| err.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub(super) fn capture_probe_pane(socket_name: &str) -> Result<String, String> {
    run_tmux_output(
        socket_name,
        &["capture-pane", "-p", "-t", "pad-probe:0.0", "-S", "-6"],
    )
}

pub(super) fn run_tmux_output(socket_name: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .arg("-L")
        .arg(socket_name)
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub(in crate::tmux_capabilities) fn now_stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
