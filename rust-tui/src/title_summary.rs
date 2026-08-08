mod normalize {
    use crate::text_normalize::collapse_whitespace;

    const MAX_TITLE_CHARS: usize = 60;

    pub fn normalize_generated_title(raw: &str) -> Option<String> {
        let single_line = raw.trim().lines().next()?.trim();
        if single_line.is_empty() {
            return None;
        }

        let mut normalized = collapse_whitespace(single_line);
        normalized = strip_known_prefix(&normalized).to_string();

        while let Some(stripped) = strip_matching_wrappers(&normalized) {
            normalized = stripped.to_string();
        }

        if normalized.is_empty() {
            return None;
        }

        let mut clipped = String::new();
        for (idx, ch) in normalized.chars().enumerate() {
            if idx >= MAX_TITLE_CHARS {
                break;
            }
            clipped.push(ch);
        }

        let clipped = clipped.trim();
        if clipped.is_empty() {
            None
        } else {
            Some(clipped.to_string())
        }
    }

    fn strip_known_prefix(value: &str) -> &str {
        let trimmed = value.trim();
        for prefix in ["title:", "Title:", "标题:", "題名:", "标题：", "題名："] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let rest = rest.trim();
                if !rest.is_empty() {
                    return rest;
                }
            }
        }
        trimmed
    }

    fn strip_matching_wrappers(value: &str) -> Option<&str> {
        let trimmed = value.trim();
        let pairs = [
            ('"', '"'),
            ('\'', '\''),
            ('`', '`'),
            ('“', '”'),
            ('‘', '’'),
            ('「', '」'),
            ('『', '』'),
            ('《', '》'),
            ('〈', '〉'),
        ];

        for (left, right) in pairs {
            if trimmed.starts_with(left) && trimmed.ends_with(right) && trimmed.len() > 1 {
                let start = left.len_utf8();
                let end = trimmed.len().saturating_sub(right.len_utf8());
                if start < end {
                    return Some(trimmed[start..end].trim());
                }
            }
        }
        None
    }
}
mod prompt {
    use super::util::truncate_for_log;
    use crate::model::PreviewTurn;

    const MAX_ASSISTANT_SNIPPET_CHARS: usize = 300;

    pub(super) fn build_summary_prompt(turns: &[PreviewTurn]) -> String {
        let mut prompt = String::from(
            "Generate one concise title for this coding conversation.\n\
    Return exactly one plain-text line in the conversation's main language.\n\
    Do not use quotes, markdown, prefixes, or explanations.\n\
    Prefer 4-10 words when possible.\n\nConversation:\n",
        );

        for (idx, turn) in turns.iter().enumerate() {
            let turn_no = idx + 1;
            prompt.push_str(&format!("User {turn_no}: {}\n", turn.question.trim()));
            if let Some(answer) = turn
                .answer
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
            {
                prompt.push_str(&format!(
                    "Assistant {turn_no}: {}\n",
                    truncate_for_log(answer, MAX_ASSISTANT_SNIPPET_CHARS)
                ));
            }
        }

        prompt
    }
}
mod request;
mod response {
    use serde_json::Value;

    pub(super) fn extract_response_text(payload: &Value) -> Option<String> {
        if let Some(text) = payload.get("output_text").and_then(Value::as_str) {
            return Some(text.to_string());
        }

        if let Some(text) = payload
            .pointer("/choices/0/message/content")
            .and_then(extract_content_text)
        {
            return Some(text);
        }

        payload
            .get("output")
            .and_then(Value::as_array)
            .and_then(|items| {
                let mut collected = String::new();
                let mut has_text = false;
                for item in items {
                    if let Some(content) = item.get("content").and_then(Value::as_array) {
                        for block in content {
                            if let Some(text) = block.get("text").and_then(Value::as_str) {
                                push_response_text(&mut collected, &mut has_text, text.trim());
                            }
                        }
                    }
                }
                if has_text {
                    Some(collected)
                } else {
                    None
                }
            })
    }

    pub(super) fn extract_error_text(payload: &Value) -> Option<String> {
        payload
            .pointer("/error/message")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| {
                payload
                    .get("message")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
    }

    fn extract_content_text(value: &Value) -> Option<String> {
        if let Some(text) = value.as_str() {
            return Some(text.to_string());
        }
        if let Some(items) = value.as_array() {
            let mut collected = String::new();
            let mut has_text = false;
            for item in items {
                if let Some(text) = item.get("text").and_then(Value::as_str) {
                    push_response_text(&mut collected, &mut has_text, text.trim());
                }
            }
            if has_text {
                return Some(collected);
            }
        }
        None
    }

    fn push_response_text(out: &mut String, has_text: &mut bool, text: &str) {
        if *has_text {
            out.push('\n');
        }
        out.push_str(text);
        *has_text = true;
    }
}
mod types {
    pub(super) const TITLE_SUMMARY_MODEL: &str = "gpt-5.1-codex-mini";

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SummaryWireApi {
        Responses,
        Chat,
    }

    impl SummaryWireApi {
        pub fn from_config(value: &str) -> Self {
            if value.trim().eq_ignore_ascii_case("chat") {
                Self::Chat
            } else {
                Self::Responses
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TitleSummaryResult {
        pub request_key: String,
        pub session_id: String,
        pub turn_count: usize,
        pub title: Option<String>,
        pub error: Option<String>,
    }
}
mod util {
    pub(super) fn truncate_for_log(value: &str, max_chars: usize) -> String {
        let mut truncated = String::new();
        for (idx, ch) in value.chars().enumerate() {
            if idx >= max_chars {
                truncated.push_str("...");
                break;
            }
            truncated.push(ch);
        }
        truncated
    }
}
mod window {
    use crate::model::PreviewTurn;

    const INITIAL_TURN_THRESHOLD: usize = 3;
    const REFRESH_INTERVAL_TURNS: usize = 6;
    const INITIAL_WINDOW_TURNS: usize = 3;
    const REFRESH_WINDOW_TURNS: usize = 6;

    pub fn is_enabled(config: &crate::theme::CodexConfig) -> bool {
        config.title_summary
    }

    pub fn should_refresh_title(turn_count: usize, generated_turn_count: Option<usize>) -> bool {
        if turn_count < INITIAL_TURN_THRESHOLD {
            return false;
        }

        match generated_turn_count {
            Some(previous) if previous >= INITIAL_TURN_THRESHOLD => {
                turn_count >= previous.saturating_add(REFRESH_INTERVAL_TURNS)
            }
            _ => true,
        }
    }

    pub fn select_turn_window(
        turns: &[PreviewTurn],
        generated_turn_count: Option<usize>,
    ) -> Vec<PreviewTurn> {
        let limit = if matches!(generated_turn_count, Some(count) if count >= INITIAL_TURN_THRESHOLD)
        {
            REFRESH_WINDOW_TURNS
        } else {
            INITIAL_WINDOW_TURNS
        };

        let mut selected = turns
            .iter()
            .filter(|turn| !turn.question.trim().is_empty())
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        selected.reverse();
        selected
    }
}

pub use normalize::normalize_generated_title;
pub use request::request_title_summary;
pub use types::{SummaryWireApi, TitleSummaryResult};
pub use window::{is_enabled, select_turn_window, should_refresh_title};

#[cfg(test)]
mod tests;
