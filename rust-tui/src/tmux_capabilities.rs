use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod report;
#[cfg(test)]
mod tests;
mod version;

pub use report::{TmuxCapabilities, TmuxProbeReport};
use version::parse_tmux_version;
pub use version::TmuxVersion;

pub fn read_tmux_version() -> Result<(String, Option<TmuxVersion>), String> {
    let output = Command::new("tmux")
        .arg("-V")
        .output()
        .map_err(|err| format!("failed to run `tmux -V`: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "`tmux -V` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((raw.clone(), parse_tmux_version(&raw)))
}

pub fn probe_tmux_capabilities() -> Result<TmuxProbeReport, String> {
    let (version_raw, version) = read_tmux_version()?;
    let socket_name = format!("pad-probe-{}-{}", std::process::id(), now_stamp());
    let mut notes = Vec::new();

    start_probe_server(&socket_name)
        .map_err(|err| format!("failed to start temporary tmux probe server: {err}"))?;

    let capabilities = TmuxCapabilities {
        pane_metadata_formats: probe_pane_metadata_formats(&socket_name, &mut notes),
        display_message_formats: probe_display_message_formats(&socket_name, &mut notes),
        root_key_table: probe_root_key_table(&socket_name, &mut notes),
        literal_send_keys: probe_literal_send_keys(&socket_name, &mut notes),
        bracketed_paste: probe_bracketed_paste(&socket_name, &mut notes),
        control_mode_flags: probe_control_mode_flags(&socket_name, &mut notes),
        focus_events: probe_focus_events(&socket_name, &mut notes),
    };

    let _ = stop_probe_server(&socket_name);

    Ok(TmuxProbeReport {
        version_raw,
        version,
        capabilities,
        notes,
    })
}

fn start_probe_server(socket_name: &str) -> Result<(), String> {
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

fn stop_probe_server(socket_name: &str) -> Result<(), String> {
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

fn probe_pane_metadata_formats(socket_name: &str, notes: &mut Vec<String>) -> bool {
    let output = run_tmux_output(
        socket_name,
        &[
            "list-panes",
            "-a",
            "-F",
            "#{session_name}|#{pane_id}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}",
        ],
    );
    match output {
        Ok(stdout) => {
            let line = stdout.lines().next().unwrap_or_default();
            let parts = line.split('|').collect::<Vec<_>>();
            let ok = parts.len() == 5
                && parts[0] == "pad-probe"
                && parts[1].starts_with('%')
                && !parts[2].is_empty()
                && !parts[3].is_empty()
                && !parts[4].is_empty();
            if !ok {
                notes.push(format!(
                    "pane metadata probe returned unexpected output: {line}"
                ));
            }
            ok
        }
        Err(err) => {
            notes.push(format!("pane metadata probe failed: {err}"));
            false
        }
    }
}

fn probe_display_message_formats(socket_name: &str, notes: &mut Vec<String>) -> bool {
    let output = run_tmux_output(
        socket_name,
        &[
            "display-message",
            "-p",
            "-t",
            "pad-probe:0.0",
            "#{session_name}:#{window_index}|#{window_zoomed_flag}|#{pane_id}",
        ],
    );
    match output {
        Ok(stdout) => {
            let value = stdout.trim();
            let parts = value.split('|').collect::<Vec<_>>();
            let ok = parts.len() == 3
                && parts[0] == "pad-probe:0"
                && matches!(parts[1], "0" | "1")
                && parts[2].starts_with('%');
            if !ok {
                notes.push(format!(
                    "display-message probe returned unexpected output: {value}"
                ));
            }
            ok
        }
        Err(err) => {
            notes.push(format!("display-message probe failed: {err}"));
            false
        }
    }
}

fn probe_root_key_table(socket_name: &str, notes: &mut Vec<String>) -> bool {
    match run_tmux_output(socket_name, &["list-keys", "-T", "root"]) {
        Ok(_) => true,
        Err(err) => {
            notes.push(format!("root key-table probe failed: {err}"));
            false
        }
    }
}

fn probe_literal_send_keys(socket_name: &str, notes: &mut Vec<String>) -> bool {
    let probe = "pad-literal-probe";
    if let Err(err) = run_tmux_output(socket_name, &["send-keys", "-t", "pad-probe:0.0", "C-c"]) {
        notes.push(format!("literal send-keys reset failed: {err}"));
        return false;
    }
    match run_tmux_output(
        socket_name,
        &["send-keys", "-l", "-t", "pad-probe:0.0", probe],
    ) {
        Ok(_) => {}
        Err(err) => {
            notes.push(format!("literal send-keys probe failed: {err}"));
            return false;
        }
    }

    let ok = capture_probe_pane(socket_name)
        .map(|capture| capture.contains(probe))
        .unwrap_or(false);
    let _ = run_tmux_output(socket_name, &["send-keys", "-t", "pad-probe:0.0", "C-u"]);
    if !ok {
        notes.push("literal send-keys probe did not appear in pane capture".to_string());
    }
    ok
}

fn probe_bracketed_paste(socket_name: &str, notes: &mut Vec<String>) -> bool {
    let probe = "pad-bracketed-paste-probe";
    if let Err(err) = run_tmux_output(socket_name, &["send-keys", "-t", "pad-probe:0.0", "C-c"]) {
        notes.push(format!("bracketed paste reset failed: {err}"));
        return false;
    }
    if let Err(err) = run_tmux_output(socket_name, &["set-buffer", "-b", "pad-probe", probe]) {
        notes.push(format!("set-buffer probe failed: {err}"));
        return false;
    }
    match run_tmux_output(
        socket_name,
        &[
            "paste-buffer",
            "-d",
            "-p",
            "-b",
            "pad-probe",
            "-t",
            "pad-probe:0.0",
        ],
    ) {
        Ok(_) => {}
        Err(err) => {
            notes.push(format!("bracketed paste probe failed: {err}"));
            return false;
        }
    }

    let ok = capture_probe_pane(socket_name)
        .map(|capture| capture.contains(probe))
        .unwrap_or(false);
    let _ = run_tmux_output(socket_name, &["send-keys", "-t", "pad-probe:0.0", "C-u"]);
    if !ok {
        notes.push("bracketed paste probe did not appear in pane capture".to_string());
    }
    ok
}

fn probe_control_mode_flags(socket_name: &str, notes: &mut Vec<String>) -> bool {
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

fn probe_focus_events(socket_name: &str, notes: &mut Vec<String>) -> bool {
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

fn capture_probe_pane(socket_name: &str) -> Result<String, String> {
    run_tmux_output(
        socket_name,
        &["capture-pane", "-p", "-t", "pad-probe:0.0", "-S", "-6"],
    )
}

fn run_tmux_output(socket_name: &str, args: &[&str]) -> Result<String, String> {
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

fn now_stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
