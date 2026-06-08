use super::SessionPaneInfo;
use std::error::Error;
use std::process::Command;

pub fn capture_pane_tail(pane_id: &str, lines: usize) -> Result<String, Box<dyn Error>> {
    let output = Command::new("tmux")
        .args([
            "capture-pane",
            "-p",
            "-t",
            pane_id,
            "-S",
            &format!("-{}", lines),
        ])
        .output()?;

    if output.status.success() {
        return Ok(crate::scanner::strip_ansi(&String::from_utf8_lossy(
            &output.stdout,
        )));
    }

    Err(format!(
        "tmux capture-pane failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}

pub fn session_exists(session_name: &str) -> Result<bool, Box<dyn Error>> {
    let output = Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .output()?;
    Ok(output.status.success())
}

pub fn list_session_panes(session_name: &str) -> Result<Vec<SessionPaneInfo>, Box<dyn Error>> {
    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-t",
            session_name,
            "-F",
            "#{pane_id}|#{pane_pid}|#{pane_current_command}",
        ])
        .output()?;

    if output.status.success() {
        return Ok(parse_session_panes_output(&String::from_utf8_lossy(
            &output.stdout,
        )));
    }

    Err(format!(
        "tmux list-panes failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}

fn parse_session_panes_output(output: &str) -> Vec<SessionPaneInfo> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '|');
            let pane_id = parts.next()?.trim();
            if pane_id.is_empty() {
                return None;
            }
            let pid = parts
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .and_then(|value| value.parse::<u32>().ok());
            let command = parts.next().unwrap_or_default().trim().to_string();
            Some(SessionPaneInfo {
                pane_id: pane_id.to_string(),
                pid,
                command,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_session_panes_output;

    #[test]
    fn parse_session_panes_output_extracts_pane_pid_and_command() {
        let panes = parse_session_panes_output("%1|1234|pad\n%2||zsh\n");
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].pane_id, "%1");
        assert_eq!(panes[0].pid, Some(1234));
        assert_eq!(panes[0].command, "pad");
        assert_eq!(panes[1].pane_id, "%2");
        assert_eq!(panes[1].pid, None);
        assert_eq!(panes[1].command, "zsh");
    }
}
