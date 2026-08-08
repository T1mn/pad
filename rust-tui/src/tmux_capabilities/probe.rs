use super::TmuxCapabilities;

mod control;
mod formats;
mod input {
    use super::runtime::{capture_probe_pane, run_tmux_output};

    pub(super) fn probe_literal_send_keys(socket_name: &str, notes: &mut Vec<String>) -> bool {
        let probe = "pad-literal-probe";
        if let Err(err) = run_tmux_output(socket_name, &["send-keys", "-t", "pad-probe:0.0", "C-c"])
        {
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

    pub(super) fn probe_bracketed_paste(socket_name: &str, notes: &mut Vec<String>) -> bool {
        let probe = "pad-bracketed-paste-probe";
        if let Err(err) = run_tmux_output(socket_name, &["send-keys", "-t", "pad-probe:0.0", "C-c"])
        {
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
}
mod runtime {
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub(in crate::tmux_capabilities) fn start_probe_server(
        socket_name: &str,
    ) -> Result<(), String> {
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
}

use control::{probe_control_mode_flags, probe_focus_events, probe_root_key_table};
use formats::{probe_display_message_formats, probe_pane_metadata_formats};
use input::{probe_bracketed_paste, probe_literal_send_keys};
pub(super) use runtime::{now_stamp, start_probe_server, stop_probe_server};

pub(super) fn probe_tmux_capabilities_with_socket(
    socket_name: &str,
    notes: &mut Vec<String>,
) -> TmuxCapabilities {
    TmuxCapabilities {
        pane_metadata_formats: probe_pane_metadata_formats(socket_name, notes),
        display_message_formats: probe_display_message_formats(socket_name, notes),
        root_key_table: probe_root_key_table(socket_name, notes),
        literal_send_keys: probe_literal_send_keys(socket_name, notes),
        bracketed_paste: probe_bracketed_paste(socket_name, notes),
        control_mode_flags: probe_control_mode_flags(socket_name, notes),
        focus_events: probe_focus_events(socket_name, notes),
    }
}
