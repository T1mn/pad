use super::client::bearer_get;
use super::types::ProbeOutcome;

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
        let url = format!("{candidate}/models");
        let started_at = std::time::Instant::now();
        let response = match bearer_get(client, &url, credential).send().await {
            Ok(response) => response,
            Err(err) => {
                last_message = format!("Codex relay probe failed for {}: {}", candidate, err);
                continue;
            }
        };

        let latency_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let status = response.status().as_u16();
        last_http_status = Some(status);

        let payload = match response.json::<serde_json::Value>().await {
            Ok(payload) => payload,
            Err(err) => {
                last_message = format!(
                    "Codex relay probe at {} returned a non-JSON /models response (HTTP {}): {}",
                    candidate, status, err
                );
                continue;
            }
        };

        let Some(models) = payload.get("data").and_then(|value| value.as_array()) else {
            last_message = format!(
                "Codex relay probe at {} returned an unexpected /models payload (HTTP {})",
                candidate, status
            );
            continue;
        };

        return (
            true,
            Some(status),
            Some(latency_ms),
            reachable_codex_message(&candidate, &input_base, status, latency_ms, models.len()),
        );
    }

    if last_message.is_empty() {
        last_message = "Codex relay probe failed".to_string();
    }

    (false, last_http_status, None, last_message)
}

fn reachable_codex_message(
    candidate: &str,
    input_base: &str,
    status: u16,
    latency_ms: u64,
    model_count: usize,
) -> String {
    if candidate != input_base {
        format!(
            "Reachable for Codex relay probe via normalized API base {}: HTTP {} in {} ms ({} models)",
            candidate, status, latency_ms, model_count
        )
    } else {
        format!(
            "Reachable for Codex relay probe: HTTP {} in {} ms ({} models)",
            status, latency_ms, model_count
        )
    }
}
