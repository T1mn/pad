use crate::app::CopyToast;
use std::time::{Duration, Instant};

pub(super) fn show_action_toast(slot: &mut Option<CopyToast>, title: &str, content: &str) {
    *slot = Some(CopyToast {
        title: title.to_string(),
        content_preview: summarize_copy_preview(content, 24),
        expires_at: Instant::now() + Duration::from_millis(1800),
    });
}

pub(super) fn copy_toast_expired(slot: &Option<CopyToast>) -> bool {
    slot.as_ref()
        .is_some_and(|toast| Instant::now() >= toast.expires_at)
}

pub(super) fn summarize_copy_preview(text: &str, max_chars: usize) -> String {
    let condensed = collapse_whitespace(text);
    if condensed.is_empty() {
        return String::from("-");
    }

    let mut preview = String::new();
    for (idx, ch) in condensed.chars().enumerate() {
        if idx >= max_chars {
            preview.push_str("...");
            return preview;
        }
        preview.push(ch);
    }
    preview
}

fn collapse_whitespace(text: &str) -> String {
    let mut condensed = String::new();
    for part in text.split_whitespace() {
        if !condensed.is_empty() {
            condensed.push(' ');
        }
        condensed.push_str(part);
    }
    condensed
}
