pub(in crate::chat::providers::telegram::hooks) struct ResolvedPendingResult {
    pub(in crate::chat::providers::telegram::hooks) text: Option<String>,
    pub(in crate::chat::providers::telegram::hooks) source: &'static str,
    pub(in crate::chat::providers::telegram::hooks) char_count: usize,
}

impl ResolvedPendingResult {
    pub(super) fn new(text: Option<String>, source: &'static str) -> Self {
        let char_count = text
            .as_ref()
            .map(|value| value.chars().count())
            .unwrap_or(0);
        Self {
            text,
            source,
            char_count,
        }
    }
}

pub(in crate::chat::providers::telegram::hooks) struct PendingCompletionOutcome {
    pub(in crate::chat::providers::telegram::hooks) source: &'static str,
    pub(in crate::chat::providers::telegram::hooks) char_count: usize,
    pub(in crate::chat::providers::telegram::hooks) error: Option<String>,
}

impl PendingCompletionOutcome {
    pub(super) fn delivered(resolved: &ResolvedPendingResult) -> Self {
        Self {
            source: resolved.source,
            char_count: resolved.char_count,
            error: None,
        }
    }

    pub(super) fn deferred(resolved: &ResolvedPendingResult, error: String) -> Self {
        Self {
            source: resolved.source,
            char_count: resolved.char_count,
            error: Some(error),
        }
    }
}
