use std::time::Duration;

use tokio::process::{Child, Command as TokioCommand};

pub(super) async fn spawn_control_client(
    session_name: &str,
) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
    match spawn_control_client_once(session_name, true).await? {
        Some(child) => Ok(child),
        None => {
            log_debug!("tmux_pipe: control-mode flags unsupported, retrying without -f");
            spawn_control_client_once(session_name, false)
                .await?
                .ok_or_else(|| "tmux control mode exited immediately without flags".into())
        }
    }
}

async fn spawn_control_client_once(
    session_name: &str,
    use_flags: bool,
) -> Result<Option<Child>, Box<dyn std::error::Error + Send + Sync>> {
    let mut command = TokioCommand::new("tmux");
    command.args(["-C", "attach-session", "-t", session_name]);
    if use_flags {
        // no-output disables noisy %output notifications; pad only needs
        // structural/session/mode events from control mode.
        command.args(["-f", "read-only,ignore-size,no-output"]);
    }

    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // If the task/runtime is cancelled before we explicitly kill the client,
        // do not leave a detached tmux control-mode process behind.
        .kill_on_drop(true)
        .spawn()?;

    tokio::time::sleep(Duration::from_millis(150)).await;

    match child.try_wait()? {
        Some(_) => {
            let output = child.wait_with_output().await?;
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if use_flags && should_retry_without_flags(&stderr) {
                Ok(None)
            } else {
                Err(format!(
                    "tmux control mode exited early{}{}",
                    if stderr.is_empty() { "" } else { ": " },
                    stderr
                )
                .into())
            }
        }
        None => Ok(Some(child)),
    }
}

fn should_retry_without_flags(stderr: &str) -> bool {
    stderr.contains("unknown option -- f")
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
