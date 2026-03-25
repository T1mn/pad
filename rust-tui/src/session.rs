use std::error::Error;
use std::process::Command;

/// Create a new tmux session in the given path with an agent command.
/// After creation, switches the tmux client to the new session and installs
/// F12/Ctrl+Q bindings so the user can return to the pad session.
pub fn create_session_with_agent(path: &str, agent_cmd: &str) -> Result<(), Box<dyn Error>> {
    let session_name = path
        .replace('/', "_")
        .replace('.', "_")
        .replace('~', "home");

    log_debug!(
        "session: create_session_with_agent path={} cmd={} session_name={}",
        path,
        agent_cmd,
        session_name
    );

    let check = Command::new("tmux")
        .args(["has-session", "-t", &session_name])
        .output()?;

    if check.status.success() {
        log_debug!(
            "session: session '{}' already exists, opening new window",
            session_name
        );
        // Session exists, open a new window with the agent
        let out = Command::new("tmux")
            .args(["new-window", "-t", &session_name, "-c", path, agent_cmd])
            .output()?;
        log_debug!(
            "session: new-window status={} stderr={}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    } else {
        log_debug!("session: creating new session '{}'", session_name);
        // Create new session with agent
        let out = Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-s",
                &session_name,
                "-c",
                path,
                agent_cmd,
            ])
            .output()?;
        log_debug!(
            "session: new-session status={} stderr={}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    // Get the pad pane id so we can install a return binding
    let pad_pane = std::env::var("TMUX_PANE").ok();
    let pad_win = pad_pane.as_deref().and_then(|pane_id| {
        Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                pane_id,
                "#{session_name}:#{window_index}",
            ])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    });

    log_debug!("session: pad_pane={:?} pad_win={:?}", pad_pane, pad_win);

    // Install F12/Ctrl+Q bindings in the new session so user can return to pad
    if let (Some(pane_id), Some(win_target)) = (pad_pane, pad_win) {
        let return_cmd = format!(
            "tmux select-window -t '{}' && tmux select-pane -t '{}'",
            win_target, pane_id
        );
        let bind_result = Command::new("tmux")
            .args(["bind-key", "-T", "root", "F12", "run-shell", &return_cmd])
            .output();
        log_debug!(
            "session: installed F12 return binding -> {} (result={:?})",
            return_cmd,
            bind_result.map(|o| o.status)
        );

        let _ = Command::new("tmux")
            .args(["bind-key", "-T", "root", "C-q", "run-shell", &return_cmd])
            .output();
    } else {
        log_debug!("session: TMUX_PANE not set, skipping F12 binding install");
    }

    // Switch the tmux client to the target session
    let sw = Command::new("tmux")
        .args(["switch-client", "-t", &session_name])
        .output();
    log_debug!(
        "session: switch-client -t {} result={:?}",
        session_name,
        sw.map(|o| o.status)
    );

    Ok(())
}
