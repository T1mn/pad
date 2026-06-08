use super::run_tmux_with_output;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn dispatch_prompt(pane_id: &str, prompt: &str) -> Result<(), Box<dyn Error>> {
    if is_multiline(prompt) {
        return dispatch_pasted_prompt(pane_id, prompt);
    }

    match dispatch_literal_prompt(pane_id, prompt) {
        Ok(()) => Ok(()),
        Err(err) => {
            log_debug!(
                "tmux_dispatch: literal send failed pane={} len={} err={}, falling back to paste",
                pane_id,
                prompt.chars().count(),
                err
            );
            dispatch_pasted_prompt(pane_id, prompt)
        }
    }
}

fn dispatch_pasted_prompt(pane_id: &str, prompt: &str) -> Result<(), Box<dyn Error>> {
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

fn dispatch_literal_prompt(pane_id: &str, prompt: &str) -> Result<(), Box<dyn Error>> {
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

fn is_multiline(prompt: &str) -> bool {
    prompt.contains('\n') || prompt.contains('\r')
}

fn split_literal_chunks(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 || text.is_empty() {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut count = 0usize;

    for ch in text.chars() {
        current.push(ch);
        count += 1;
        if count >= max_chars {
            chunks.push(std::mem::take(&mut current));
            count = 0;
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    if chunks.is_empty() {
        chunks.push(String::new());
    }

    chunks
}

fn submit_delay_for(prompt: &str, pasted: bool) -> Duration {
    let base_ms = if pasted { 120u64 } else { 80u64 };
    let extra_ms = ((prompt.chars().count() as u64) / 32).saturating_mul(12);
    Duration::from_millis((base_ms + extra_ms).min(320))
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{split_literal_chunks, submit_delay_for};

    #[test]
    fn split_literal_chunks_preserves_text() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let chunks = split_literal_chunks(text, 5);
        assert_eq!(chunks.join(""), text);
        assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 5));
    }

    #[test]
    fn submit_delay_grows_for_longer_prompts() {
        let short = submit_delay_for("short prompt", false);
        let long = submit_delay_for(&"x".repeat(320), false);
        assert!(long > short);
    }
}
