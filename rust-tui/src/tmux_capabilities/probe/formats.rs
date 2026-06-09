use super::runtime::run_tmux_output;

pub(super) fn probe_pane_metadata_formats(socket_name: &str, notes: &mut Vec<String>) -> bool {
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
            let ok = pane_metadata_probe_output_ok(line);
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

pub(super) fn probe_display_message_formats(socket_name: &str, notes: &mut Vec<String>) -> bool {
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
            let ok = display_message_probe_output_ok(value);
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

fn pane_metadata_probe_output_ok(line: &str) -> bool {
    let Some((session_name, rest)) = line.split_once('|') else {
        return false;
    };
    let Some((pane_id, rest)) = rest.split_once('|') else {
        return false;
    };
    let Some((pane_pid, rest)) = rest.split_once('|') else {
        return false;
    };
    let Some((pane_command, pane_path)) = rest.split_once('|') else {
        return false;
    };

    session_name == "pad-probe"
        && pane_id.starts_with('%')
        && !pane_pid.is_empty()
        && !pane_command.is_empty()
        && !pane_path.is_empty()
        && !pane_path.contains('|')
}

fn display_message_probe_output_ok(value: &str) -> bool {
    let Some((target, rest)) = value.split_once('|') else {
        return false;
    };
    let Some((zoomed_flag, pane_id)) = rest.split_once('|') else {
        return false;
    };

    target == "pad-probe:0"
        && matches!(zoomed_flag, "0" | "1")
        && pane_id.starts_with('%')
        && !pane_id.contains('|')
}

#[cfg(test)]
#[path = "formats_tests.rs"]
mod tests;
