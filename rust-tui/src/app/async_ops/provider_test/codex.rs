use super::client::bearer_post_json;
use super::types::ProbeOutcome;
use serde_json::json;

mod error {
    pub(super) fn classify_error(status: u16, body: &str) -> &'static str {
        let lower = body.to_ascii_lowercase();
        if status == 401 || status == 403 {
            "auth"
        } else if status == 404 {
            "not_found"
        } else if status == 408 || lower.contains("timeout") {
            "timeout"
        } else if status == 429 {
            "rate_limit"
        } else if lower.contains("model")
            && (lower.contains("not") || lower.contains("invalid") || lower.contains("unsupported"))
        {
            "model"
        } else if status >= 500 {
            "server_error"
        } else {
            "http_error"
        }
    }

    pub(super) fn truncate_message(input: &str, max_chars: usize) -> String {
        let mut out = String::new();
        for ch in input.trim().chars().take(max_chars) {
            if ch.is_control() {
                out.push(' ');
            } else {
                out.push(ch);
            }
        }
        out
    }
}
mod model {
    pub(super) fn codex_probe_model() -> String {
        std::env::var("PAD_CODEX_PROVIDER_TEST_MODEL")
            .ok()
            .filter(|model| !model.trim().is_empty())
            .or_else(read_configured_codex_model)
            .unwrap_or_else(|| "gpt-5.5".to_string())
    }

    fn read_configured_codex_model() -> Option<String> {
        let content = std::fs::read_to_string(crate::paths::pad_codex_config_path()).ok()?;
        let parsed = content.parse::<toml::Value>().ok()?;
        parsed
            .get("model")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .map(ToOwned::to_owned)
    }
}
mod response_text {
    pub(super) fn extract_response_text(payload: &serde_json::Value) -> Option<String> {
        if let Some(text) = payload.get("output_text").and_then(|value| value.as_str()) {
            return Some(text.to_string());
        }

        let mut out = String::new();
        let output = payload.get("output").and_then(|value| value.as_array())?;
        for item in output {
            let Some(content) = item.get("content").and_then(|value| value.as_array()) else {
                continue;
            };
            for content_item in content {
                if let Some(text) = content_item.get("text").and_then(|value| value.as_str()) {
                    out.push_str(text);
                }
            }
        }
        Some(out)
    }
}
mod stream;

use error::{classify_error, truncate_message};
use model::codex_probe_model;
use response_text::extract_response_text;
use stream::{read_streaming_response, StreamProbe};

const REAL_PROBE_PROMPT: &str = "请只回复 OK";
const REAL_PROBE_MAX_OUTPUT_TOKENS: u16 = 16;

pub(super) async fn probe_codex_provider(
    client: &reqwest::Client,
    base_url: &str,
    credential: Option<&str>,
) -> ProbeOutcome {
    let input_base = base_url.trim().trim_end_matches('/').to_string();
    let candidates = crate::theme::codex_api_base_candidates(base_url);
    if candidates.is_empty() {
        return (false, None, None, "Base URL is empty".to_string());
    }

    let mut last_http_status = None;
    let mut last_message = String::new();

    for candidate in candidates {
        let url = format!("{candidate}/responses");
        let started_at = std::time::Instant::now();
        let payload = json!({
            "model": codex_probe_model(),
            "input": REAL_PROBE_PROMPT,
            "instructions": "只输出两个大写字母 OK，不要解释。",
            "stream": true,
            "max_output_tokens": REAL_PROBE_MAX_OUTPUT_TOKENS,
        });
        let response = match bearer_post_json(client, &url, credential, &payload)
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                last_message = format!(
                    "Codex real chat probe failed for {}: network · {}",
                    candidate, err
                );
                continue;
            }
        };

        let status = response.status().as_u16();
        last_http_status = Some(status);
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            last_message = format!(
                "Codex real chat probe at {} failed: {} · HTTP {} · {}",
                candidate,
                classify_error(status, &body),
                status,
                truncate_message(&body, 220)
            );
            continue;
        }

        if content_type.contains("text/event-stream") {
            match read_streaming_response(response, started_at).await {
                StreamProbe::Ok {
                    first_output_ms,
                    total_ms,
                    text,
                } => {
                    return (
                        true,
                        Some(status),
                        Some(first_output_ms),
                        reachable_codex_message(
                            &candidate,
                            &input_base,
                            status,
                            first_output_ms,
                            total_ms,
                            &text,
                        ),
                    );
                }
                StreamProbe::Failed {
                    category,
                    total_ms,
                    message,
                } => {
                    last_message = format!(
                        "Codex real chat probe at {} failed: {} · HTTP {} · total {} ms · {}",
                        candidate, category, status, total_ms, message
                    );
                    continue;
                }
            }
        }

        if content_type.contains("application/json") {
            let total_ms = elapsed_ms(started_at);
            let payload = match response.json::<serde_json::Value>().await {
                Ok(payload) => payload,
                Err(err) => {
                    last_message = format!(
                        "Codex real chat probe at {} failed: non_json · HTTP {} · {}",
                        candidate, status, err
                    );
                    continue;
                }
            };
            if let Some(text) =
                extract_response_text(&payload).filter(|text| !text.trim().is_empty())
            {
                return (
                    true,
                    Some(status),
                    Some(total_ms),
                    reachable_codex_message(
                        &candidate,
                        &input_base,
                        status,
                        total_ms,
                        total_ms,
                        &text,
                    ),
                );
            }
            last_message = format!(
                "Codex real chat probe at {} failed: no_output · HTTP {} · total {} ms",
                candidate, status, total_ms
            );
            continue;
        }

        let total_ms = elapsed_ms(started_at);
        let preview = response.text().await.unwrap_or_default();
        last_message = format!(
            "Codex real chat probe at {} failed: unexpected_content_type · HTTP {} · total {} ms · {}",
            candidate,
            status,
            total_ms,
            truncate_message(&preview, 180)
        );
    }

    if last_message.is_empty() {
        last_message = "Codex real chat probe failed".to_string();
    }

    (false, last_http_status, None, last_message)
}

fn reachable_codex_message(
    candidate: &str,
    input_base: &str,
    status: u16,
    first_output_ms: u64,
    total_ms: u64,
    text: &str,
) -> String {
    let normalized = if candidate != input_base {
        format!(" via normalized API base {candidate}")
    } else {
        String::new()
    };
    format!(
        "Codex real chat OK{}: HTTP {} · first output {} ms · complete {} ms · reply {:?}",
        normalized,
        status,
        first_output_ms,
        total_ms,
        truncate_message(text, 40)
    )
}

pub(super) fn elapsed_ms(started_at: std::time::Instant) -> u64 {
    started_at.elapsed().as_millis().min(u64::MAX as u128) as u64
}
