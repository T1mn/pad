use super::runtime::{capture_probe_pane, run_tmux_output};

pub(super) fn probe_literal_send_keys(socket_name: &str, notes: &mut Vec<String>) -> bool {
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

pub(super) fn probe_bracketed_paste(socket_name: &str, notes: &mut Vec<String>) -> bool {
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
