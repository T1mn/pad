use crate::model::PreviewTurn;
use serde_json::{json, Value};

pub const TITLE_SUMMARY_MODEL: &str = "gpt-5.1-codex-mini";
const INITIAL_TURN_THRESHOLD: usize = 3;
const REFRESH_INTERVAL_TURNS: usize = 6;
const INITIAL_WINDOW_TURNS: usize = 3;
const REFRESH_WINDOW_TURNS: usize = 6;
const MAX_TITLE_CHARS: usize = 60;
const MAX_ASSISTANT_SNIPPET_CHARS: usize = 300;

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
    let limit = if matches!(generated_turn_count, Some(count) if count >= INITIAL_TURN_THRESHOLD) {
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

pub fn normalize_generated_title(raw: &str) -> Option<String> {
    let single_line = raw.trim().lines().next()?.trim();
    if single_line.is_empty() {
        return None;
    }

    let mut normalized = single_line.split_whitespace().collect::<Vec<_>>().join(" ");

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

pub async fn request_title_summary(
    client: &reqwest::Client,
    base_url: &str,
    token: Option<&str>,
    wire_api: SummaryWireApi,
    turns: &[PreviewTurn],
) -> Result<String, String> {
    let prompt = build_summary_prompt(turns);
    let candidates = crate::theme::codex_api_base_candidates(base_url);
    if candidates.is_empty() {
        return Err("Base URL is empty".to_string());
    }

    let mut last_error = None;
    for candidate in candidates {
        let endpoint = match wire_api {
            SummaryWireApi::Responses => format!("{candidate}/responses"),
            SummaryWireApi::Chat => format!("{candidate}/chat/completions"),
        };
        let mut request = client.post(&endpoint);
        if let Some(token) = token.filter(|value| !value.trim().is_empty()) {
            request = request.bearer_auth(token);
        }

        let body = match wire_api {
            SummaryWireApi::Responses => json!({
                "model": TITLE_SUMMARY_MODEL,
                "input": prompt,
                "max_output_tokens": 32,
            }),
            SummaryWireApi::Chat => json!({
                "model": TITLE_SUMMARY_MODEL,
                "messages": [
                    {
                        "role": "user",
                        "content": prompt,
                    }
                ],
                "max_tokens": 32,
            }),
        };

        let response = match request.json(&body).send().await {
            Ok(response) => response,
            Err(err) => {
                last_error = Some(format!("request to {endpoint} failed: {err}"));
                continue;
            }
        };

        let status = response.status();
        let payload = match response.json::<Value>().await {
            Ok(payload) => payload,
            Err(err) => {
                last_error = Some(format!(
                    "non-JSON response from {endpoint} (HTTP {}): {err}",
                    status
                ));
                continue;
            }
        };

        if !status.is_success() {
            let detail = extract_error_text(&payload)
                .unwrap_or_else(|| truncate_for_log(&payload.to_string(), 240));
            last_error = Some(format!("{endpoint} returned HTTP {}: {}", status, detail));
            continue;
        }

        let Some(text) = extract_response_text(&payload) else {
            last_error = Some(format!(
                "{endpoint} returned HTTP {} without usable text",
                status
            ));
            continue;
        };

        let Some(title) = normalize_generated_title(&text) else {
            last_error = Some(format!("{endpoint} returned an empty title"));
            continue;
        };

        return Ok(title);
    }

    Err(last_error.unwrap_or_else(|| "title summary request failed".to_string()))
}

fn build_summary_prompt(turns: &[PreviewTurn]) -> String {
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

fn extract_response_text(payload: &Value) -> Option<String> {
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
            let mut collected = Vec::new();
            for item in items {
                if let Some(content) = item.get("content").and_then(Value::as_array) {
                    for block in content {
                        if let Some(text) = block.get("text").and_then(Value::as_str) {
                            collected.push(text.trim());
                        }
                    }
                }
            }
            if collected.is_empty() {
                None
            } else {
                Some(collected.join("\n"))
            }
        })
}

fn extract_content_text(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if let Some(items) = value.as_array() {
        let mut collected = Vec::new();
        for item in items {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                collected.push(text.trim());
            }
        }
        if !collected.is_empty() {
            return Some(collected.join("\n"));
        }
    }
    None
}

fn extract_error_text(payload: &Value) -> Option<String> {
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

fn truncate_for_log(value: &str, max_chars: usize) -> String {
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

#[cfg(test)]
mod tests {
    use super::{
        normalize_generated_title, select_turn_window, should_refresh_title, SummaryWireApi,
    };
    use crate::model::PreviewTurn;

    #[test]
    fn responses_is_default_wire_api() {
        assert_eq!(SummaryWireApi::from_config(""), SummaryWireApi::Responses);
        assert_eq!(
            SummaryWireApi::from_config("responses"),
            SummaryWireApi::Responses
        );
        assert_eq!(SummaryWireApi::from_config("chat"), SummaryWireApi::Chat);
    }

    #[test]
    fn title_refresh_triggers_after_initial_threshold() {
        assert!(!should_refresh_title(2, None));
        assert!(should_refresh_title(3, None));
        assert!(should_refresh_title(11, None));
        assert!(!should_refresh_title(8, Some(3)));
        assert!(should_refresh_title(9, Some(3)));
    }

    #[test]
    fn initial_window_uses_three_turns_in_chronological_order() {
        let turns = vec![
            PreviewTurn {
                question: "third".into(),
                answer: Some("c".into()),
            },
            PreviewTurn {
                question: "second".into(),
                answer: Some("b".into()),
            },
            PreviewTurn {
                question: "first".into(),
                answer: Some("a".into()),
            },
        ];

        let selected = select_turn_window(&turns, None);
        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0].question, "first");
        assert_eq!(selected[2].question, "third");
    }

    #[test]
    fn refresh_window_keeps_six_newest_turns() {
        let turns = (1..=8)
            .rev()
            .map(|idx| PreviewTurn {
                question: format!("q{idx}"),
                answer: None,
            })
            .collect::<Vec<_>>();

        let selected = select_turn_window(&turns, Some(3));
        assert_eq!(selected.len(), 6);
        assert_eq!(selected[0].question, "q3");
        assert_eq!(selected[5].question, "q8");
    }

    #[test]
    fn title_normalization_trims_wrappers_and_prefixes() {
        assert_eq!(
            normalize_generated_title("Title: \"Refactor tmux popup flow\"").as_deref(),
            Some("Refactor tmux popup flow")
        );
        assert_eq!(
            normalize_generated_title("《修复会话标题自动摘要》").as_deref(),
            Some("修复会话标题自动摘要")
        );
    }
}
