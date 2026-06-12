use super::client::claude_post_json;
use super::types::ProbeOutcome;
use serde_json::json;

mod error;
mod model;
mod response_text;
mod stream;

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
