use super::util::{now_ms, submit_delay_for};
use crate::tmux_dispatch::run_tmux_with_output;
use std::error::Error;
use std::thread;

pub(super) fn dispatch_pasted_prompt(pane_id: &str, prompt: &str) -> Result<(), Box<dyn Error>> {
    let buffer_name = format!("pad-telegram-{}-{}", std::process::id(), now_ms());
    run_tmux_with_output(["set-buffer", "-b", &buffer_name, prompt])?;
    match run_tmux_with_output([
        "paste-buffer",
        "-d",
        "-p",
        "-b",
        &buffer_name,
        "-t",
        pane_id,
    ]) {
        Ok(()) => {
            log_debug!(
                "tmux_dispatch: bracketed paste succeeded pane={} buffer={}",
                pane_id,
                buffer_name
            );
        }
        Err(err) => {
            log_debug!(
                "tmux_dispatch: bracketed paste failed pane={} buffer={} err={}, falling back",
                pane_id,
                buffer_name,
                err
            );
            run_tmux_with_output(["paste-buffer", "-d", "-b", &buffer_name, "-t", pane_id])?;
        }
    }
    let submit_delay = submit_delay_for(prompt, true);
    thread::sleep(submit_delay);
    run_tmux_with_output(["send-keys", "-t", pane_id, "C-m"])?;
    log_debug!(
        "tmux_dispatch: prompt dispatched pane={} buffer={} len={} mode=paste submit=C-m delay_ms={}",
        pane_id,
        buffer_name,
        prompt.chars().count(),
        submit_delay.as_millis()
    );
    Ok(())
}
