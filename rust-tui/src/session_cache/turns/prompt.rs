pub(in crate::session_cache) fn normalize_cached_codex_prompt(
    value: Option<&str>,
    normalize_codex: bool,
) -> Option<String> {
    value.and_then(|text| {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }

        let normalized = if normalize_codex {
            crate::preview_source::codex::normalize_codex_user_text_cow(text, None)
        } else {
            std::borrow::Cow::Borrowed(text)
        };
        if normalized.is_empty() {
            None
        } else {
            Some(normalized.into_owned())
        }
    })
}
