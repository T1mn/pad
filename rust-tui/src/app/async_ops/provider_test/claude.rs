use super::client::claude_post_json;
use super::types::ProbeOutcome;
use serde_json::json;

mod error;
mod model;
mod response_text;
mod stream;

use error::{classify_error, truncate_message};
use model::claude_probe_model;
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
    let started_at = std::time::Instant::now();
    let payload = json!({
        "model": claude_probe_model(configured_model),
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
            return (
                false,
                None,
                None,
                format!("Claude real chat probe failed for {root}: network · {err}"),
            );
        }
    };

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            false,
            Some(status),
            None,
            format!(
                "Claude real chat probe at {root} failed: {} · HTTP {} · {}",
                classify_error(status, &body),
                status,
                truncate_message(&body, 220)
            ),
        );
    }

    if content_type.contains("text/event-stream") {
        return match read_streaming_response(response, started_at).await {
            StreamProbe::Ok {
                first_output_ms,
                total_ms,
                text,
            } => (
                true,
                Some(status),
                Some(first_output_ms),
                reachable_claude_message(status, first_output_ms, total_ms, &text),
            ),
            StreamProbe::Failed {
                category,
                total_ms,
                message,
            } => (
                false,
                Some(status),
                None,
                format!(
                    "Claude real chat probe at {root} failed: {category} · HTTP {status} · total {total_ms} ms · {message}"
                ),
            ),
        };
    }

    if content_type.contains("application/json") {
        let total_ms = elapsed_ms(started_at);
        let payload = match response.json::<serde_json::Value>().await {
            Ok(payload) => payload,
            Err(err) => {
                return (
                    false,
                    Some(status),
                    None,
                    format!(
                        "Claude real chat probe at {root} failed: non_json · HTTP {status} · {err}"
                    ),
                );
            }
        };
        if let Some(text) = extract_response_text(&payload).filter(|text| !text.trim().is_empty()) {
            return (
                true,
                Some(status),
                Some(total_ms),
                reachable_claude_message(status, total_ms, total_ms, &text),
            );
        }
        return (
            false,
            Some(status),
            None,
            format!("Claude real chat probe at {root} failed: no_output · HTTP {status} · total {total_ms} ms"),
        );
    }

    let total_ms = elapsed_ms(started_at);
    let preview = response.text().await.unwrap_or_default();
    (
        false,
        Some(status),
        None,
        format!(
            "Claude real chat probe at {root} failed: unexpected_content_type · HTTP {status} · total {total_ms} ms · {}",
            truncate_message(&preview, 180)
        ),
    )
}

fn reachable_claude_message(
    status: u16,
    first_output_ms: u64,
    total_ms: u64,
    text: &str,
) -> String {
    format!(
        "Claude real chat OK: HTTP {} · first output {} ms · complete {} ms · reply {:?}",
        status,
        first_output_ms,
        total_ms,
        truncate_message(text, 40)
    )
}

pub(super) fn elapsed_ms(started_at: std::time::Instant) -> u64 {
    started_at.elapsed().as_millis().min(u64::MAX as u128) as u64
}
