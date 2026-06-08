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
