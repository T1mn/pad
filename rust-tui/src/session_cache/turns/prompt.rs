pub(in crate::session_cache) fn normalize_cached_codex_prompt(
    value: Option<String>,
    normalize_codex: bool,
) -> Option<String> {
    value.and_then(|text| {
        let normalized = if normalize_codex {
            crate::preview_source::codex::normalize_codex_user_text(&text, None)
        } else {
            text.trim().to_string()
        };
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}
