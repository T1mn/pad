use std::time::Duration;

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
    crate::time::unix_now_millis()
}

#[cfg(test)]
#[path = "util_tests.rs"]
mod tests;
