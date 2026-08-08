use super::client::claude_post_json;
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
mod model;
mod response_text {
    pub(super) fn extract_response_text(payload: &serde_json::Value) -> Option<String> {
        let mut out = String::new();
        let content = payload.get("content")?.as_array()?;
        for item in content {
            if let Some(text) = item.get("text").and_then(|value| value.as_str()) {
                out.push_str(text);
            }
        }
        Some(out)
    }
}
mod stream {
    pub(super) enum StreamProbe {
        Ok {
            first_output_ms: u64,
            total_ms: u64,
            text: String,
        },
        Failed {
            category: &'static str,
            total_ms: u64,
            message: String,
        },
    }

    pub(super) async fn read_streaming_response(
        mut response: reqwest::Response,
        started_at: std::time::Instant,
    ) -> StreamProbe {
        let mut buffer = String::new();
        let mut text = String::new();
        let mut first_output_ms = None;

        loop {
            let chunk = match response.chunk().await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(err) => {
                    return StreamProbe::Failed {
                        category: "stream_read",
                        total_ms: super::elapsed_ms(started_at),
                        message: err.to_string(),
                    };
                }
            };
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(newline_idx) = buffer.find('\n') {
                let line = buffer[..newline_idx].trim().to_string();
                buffer = buffer[newline_idx + 1..].to_string();
                if !line.starts_with("data:") {
                    continue;
                }
                let data = line.trim_start_matches("data:").trim();
                if data == "[DONE]" {
                    break;
                }
                let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else {
                    continue;
                };
                if let Some(error) = event.get("error") {
                    return StreamProbe::Failed {
                        category: "api_error",
                        total_ms: super::elapsed_ms(started_at),
                        message: super::truncate_message(&error.to_string(), 220),
                    };
                }
                if event.get("type").and_then(|value| value.as_str()) == Some("error") {
                    return StreamProbe::Failed {
                        category: "api_error",
                        total_ms: super::elapsed_ms(started_at),
                        message: super::truncate_message(&event.to_string(), 220),
                    };
                }
                if let Some(delta) = event.get("delta") {
                    if let Some(delta_text) = delta.get("text").and_then(|value| value.as_str()) {
                        if !delta_text.is_empty() && first_output_ms.is_none() {
                            first_output_ms = Some(super::elapsed_ms(started_at));
                        }
                        text.push_str(delta_text);
                    }
                }
            }
        }

        match (first_output_ms, text.trim().is_empty()) {
            (Some(first_output_ms), false) => StreamProbe::Ok {
                first_output_ms,
                total_ms: super::elapsed_ms(started_at),
                text: text.trim().to_string(),
            },
            _ => StreamProbe::Failed {
                category: "no_output",
                total_ms: super::elapsed_ms(started_at),
                message: "stream completed without output text delta".to_string(),
            },
        }
    }
}

use error::{classify_error, truncate_message};
use model::claude_probe_models;
use response_text::extract_response_text;
use stream::{read_streaming_response, StreamProbe};

const REAL_PROBE_PROMPT: &str = "请只回复 OK";
const REAL_PROBE_MAX_TOKENS: u16 = 16;

pub(super) async fn probe_claude_provider(
    client: &reqwest::Client,
    base_url: &str,
    credential: Option<&str>,
    configured_model: &str,
) -> ProbeOutcome {
    let root = crate::relay::claude_base_url(base_url);
    if root.is_empty() {
        return (false, None, None, "Base URL is empty".to_string());
    }

    let url = format!("{root}/v1/messages");
    let models = claude_probe_models(configured_model);
    let mut last_http_status = None;
    let mut last_message = String::new();

    for model in &models {
        let started_at = std::time::Instant::now();
        let payload = json!({
            "model": model,
            "max_tokens": REAL_PROBE_MAX_TOKENS,
            "stream": true,
            "system": "只输出两个大写字母 OK，不要解释。",
            "messages": [{ "role": "user", "content": REAL_PROBE_PROMPT }],
        });

        let response = match claude_post_json(client, &url, credential, &payload)
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                last_message = format!("Claude real chat probe failed for {root}: network · {err}");
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
                "Claude real chat probe at {root} failed with model {model}: {} · HTTP {} · {}",
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
                        reachable_claude_message(model, status, first_output_ms, total_ms, &text),
                    );
                }
                StreamProbe::Failed {
                    category,
                    total_ms,
                    message,
                } => {
                    last_message = format!(
                        "Claude real chat probe at {root} failed with model {model}: {category} · HTTP {status} · total {total_ms} ms · {message}"
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
                        "Claude real chat probe at {root} failed with model {model}: non_json · HTTP {status} · {err}"
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
                    reachable_claude_message(model, status, total_ms, total_ms, &text),
                );
            }
            last_message = format!("Claude real chat probe at {root} failed with model {model}: no_output · HTTP {status} · total {total_ms} ms");
            continue;
        }

        let total_ms = elapsed_ms(started_at);
        let preview = response.text().await.unwrap_or_default();
        last_message = format!(
            "Claude real chat probe at {root} failed with model {model}: unexpected_content_type · HTTP {status} · total {total_ms} ms · {}",
            truncate_message(&preview, 180)
        );
    }

    (
        false,
        last_http_status,
        None,
        format!(
            "{} · tried models: {}",
            if last_message.is_empty() {
                format!("Claude real chat probe at {root} failed")
            } else {
                last_message
            },
            models.join(", ")
        ),
    )
}

fn reachable_claude_message(
    model: &str,
    status: u16,
    first_output_ms: u64,
    total_ms: u64,
    text: &str,
) -> String {
    format!(
        "Claude real chat OK: model {} · HTTP {} · first output {} ms · complete {} ms · reply {:?}",
        model,
        status,
        first_output_ms,
        total_ms,
        truncate_message(text, 40)
    )
}

pub(super) fn elapsed_ms(started_at: std::time::Instant) -> u64 {
    started_at.elapsed().as_millis().min(u64::MAX as u128) as u64
}
