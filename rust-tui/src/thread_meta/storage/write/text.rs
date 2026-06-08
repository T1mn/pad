pub(super) fn normalize_text(value: Option<&str>) -> Option<String> {
    value.and_then(clean_text)
}

fn clean_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
