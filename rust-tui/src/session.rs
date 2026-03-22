use std::error::Error;
use std::process::Command;

/// Create a new tmux session in the given path with an agent command
pub fn create_session_with_agent(path: &str, agent_cmd: &str) -> Result<(), Box<dyn Error>> {
    let session_name = path
        .replace('/', "_")
        .replace('.', "_")
        .replace('~', "home");

    let check = Command::new("tmux")
        .args(["has-session", "-t", &session_name])
        .output()?;

    if check.status.success() {
        // Session exists, open a new window with the agent
        Command::new("tmux")
            .args([
                "new-window",
                "-t",
                &session_name,
                "-c",
                path,
                agent_cmd,
            ])
            .output()?;
    } else {
        // Create new session with agent
        Command::new("tmux")
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
    }

    Ok(())
}
