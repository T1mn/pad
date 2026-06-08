use super::*;

pub(super) async fn poll_slash_reply(
    pane_id: &str,
    slash: &str,
    baseline: &str,
    deadline_ms: u64,
) -> TelegramResult<Option<String>> {
    let started = Instant::now();
    let deadline = Duration::from_millis(deadline_ms);
    let mut candidate: Option<String> = None;
    let mut stable_hits = 0usize;

    loop {
        let capture = tmux_dispatch::capture_pane_tail(pane_id, 28).map_err(telegram_error)?;
        let capture = summarize_pane_capture(&capture);
        if !capture.is_empty() && capture != baseline {
            if !capture_looks_like_echo_only(&capture, slash) {
                if candidate.as_deref() == Some(capture.as_str()) {
                    stable_hits += 1;
                } else {
                    candidate = Some(capture.clone());
                    stable_hits = 1;
                }
                if stable_hits >= 2 || started.elapsed() >= Duration::from_millis(250) {
                    return Ok(Some(capture));
                }
            } else {
                candidate = Some(capture);
            }
        }

        if started.elapsed() >= deadline {
            break;
        }
        sleep(Duration::from_millis(SLASH_POLL_INTERVAL_MS)).await;
    }

    Ok(candidate.filter(|capture| capture != baseline))
}

fn capture_looks_like_echo_only(capture: &str, slash: &str) -> bool {
    let trimmed = capture.trim();
    trimmed == slash.trim() || trimmed.ends_with(&format!("\n{}", slash.trim()))
}
