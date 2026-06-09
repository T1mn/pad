#[path = "normalize/fragments.rs"]
mod fragments;
#[path = "normalize/images.rs"]
mod images;

use self::fragments::{extract_user_shell_command_summary, strip_non_preview_codex_fragments};
use self::images::normalize_image_refs;
use std::borrow::Cow;

pub(crate) fn normalize_codex_user_text(text: &str, image_count_hint: Option<usize>) -> String {
    normalize_codex_user_text_cow(text, image_count_hint).into_owned()
}

pub(crate) fn normalize_codex_user_text_cow(
    text: &str,
    image_count_hint: Option<usize>,
) -> Cow<'_, str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Cow::Borrowed("");
    }

    if let Some(summary) = extract_user_shell_command_summary(trimmed) {
        return Cow::Owned(summary);
    }

    match strip_non_preview_codex_fragments(trimmed) {
        Cow::Borrowed(stripped) => normalize_borrowed_text(stripped, image_count_hint),
        Cow::Owned(stripped) => normalize_owned_text(stripped, image_count_hint),
    }
}

fn normalize_borrowed_text(text: &str, image_count_hint: Option<usize>) -> Cow<'_, str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Cow::Borrowed("");
    }

    if let Some(normalized) = normalize_image_refs(trimmed, image_count_hint) {
        Cow::Owned(normalized)
    } else {
        Cow::Borrowed(trimmed)
    }
}

fn normalize_owned_text(text: String, image_count_hint: Option<usize>) -> Cow<'static, str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Cow::Borrowed("");
    }

    if let Some(normalized) = normalize_image_refs(trimmed, image_count_hint) {
        Cow::Owned(normalized)
    } else {
        Cow::Owned(trimmed.to_string())
    }
}
