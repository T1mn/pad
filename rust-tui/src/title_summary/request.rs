use super::normalize_generated_title;
use super::prompt::build_summary_prompt;
use super::response::{extract_error_text, extract_response_text};
use super::types::{SummaryWireApi, TITLE_SUMMARY_MODEL};
use super::util::truncate_for_log;
use crate::model::PreviewTurn;
use serde_json::{json, Value};

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

        let body = request_body(wire_api, &prompt);
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

fn request_body(wire_api: SummaryWireApi, prompt: &str) -> Value {
    match wire_api {
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
    }
}
