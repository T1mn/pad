use super::util::{split_literal_chunks, submit_delay_for};
use crate::tmux_dispatch::run_tmux_with_output;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub(super) fn dispatch_literal_prompt(pane_id: &str, prompt: &str) -> Result<(), Box<dyn Error>> {
    let chunks = split_literal_chunks(prompt, 96);
    for (idx, chunk) in chunks.iter().enumerate() {
        run_tmux_send_literal(pane_id, chunk)?;
        if idx + 1 < chunks.len() {
            thread::sleep(Duration::from_millis(8));
        }
    }

    let submit_delay = submit_delay_for(prompt, false);
    thread::sleep(submit_delay);
    run_tmux_with_output(["send-keys", "-t", pane_id, "C-m"])?;
    log_debug!(
        "tmux_dispatch: prompt dispatched pane={} len={} mode=literal chunks={} submit=C-m delay_ms={}",
        pane_id,
        prompt.chars().count(),
        chunks.len(),
        submit_delay.as_millis()
    );
    Ok(())
}

fn run_tmux_send_literal(pane_id: &str, chunk: &str) -> Result<(), Box<dyn Error>> {
    let output = Command::new("tmux")
        .args(["send-keys", "-l", "-t", pane_id, chunk])
        .output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "tmux send-keys -l failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}
