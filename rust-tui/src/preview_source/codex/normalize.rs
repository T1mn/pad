#[path = "normalize/fragments.rs"]
mod fragments;
#[path = "normalize/images.rs"]
mod images;

use self::fragments::{extract_user_shell_command_summary, strip_non_preview_codex_fragments};
use self::images::normalize_image_refs;

pub(crate) fn normalize_codex_user_text(text: &str, image_count_hint: Option<usize>) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Some(summary) = extract_user_shell_command_summary(trimmed) {
        return summary;
    }

    let stripped_context = strip_non_preview_codex_fragments(trimmed);
    let trimmed = stripped_context.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    normalize_image_refs(trimmed, image_count_hint).unwrap_or_else(|| trimmed.to_string())
}
