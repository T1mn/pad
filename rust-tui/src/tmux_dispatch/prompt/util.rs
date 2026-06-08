use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) fn is_multiline(prompt: &str) -> bool {
    prompt.contains('\n') || prompt.contains('\r')
}

pub(super) fn split_literal_chunks(text: &str, max_chars: usize) -> Vec<String> {
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

pub(super) fn submit_delay_for(prompt: &str, pasted: bool) -> Duration {
    let base_ms = if pasted { 120u64 } else { 80u64 };
    let extra_ms = ((prompt.chars().count() as u64) / 32).saturating_mul(12);
    Duration::from_millis((base_ms + extra_ms).min(320))
}

pub(super) fn now_ms() -> u128 {
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
