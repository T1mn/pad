use super::client::bearer_get;
use super::types::ProbeOutcome;

pub(super) async fn probe_claude_provider(
    client: &reqwest::Client,
    base_url: &str,
    credential: Option<&str>,
) -> ProbeOutcome {
    let root = crate::relay::claude_base_url(base_url);
    if root.is_empty() {
        return (false, None, None, "Base URL is empty".to_string());
    }

    let url = format!("{root}/v1/models");
    let started_at = std::time::Instant::now();
    let response = match bearer_get(client, &url, credential).send().await {
        Ok(response) => response,
        Err(err) => {
            return (
                false,
                None,
                None,
                format!("Claude relay probe failed: {err}"),
            )
        }
    };

    let latency_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let status = response.status().as_u16();
    let payload = match response.json::<serde_json::Value>().await {
        Ok(payload) => payload,
        Err(err) => {
            return (
                false,
                Some(status),
                None,
                format!("Claude relay probe returned non-JSON /v1/models response: {err}"),
            );
        }
    };

    let Some(models) = payload.get("data").and_then(|value| value.as_array()) else {
        return (
            false,
            Some(status),
            Some(latency_ms),
            format!("Claude relay probe returned unexpected /v1/models payload (HTTP {status})"),
        );
    };

    (
        true,
        Some(status),
        Some(latency_ms),
        format!(
            "Reachable for Claude relay probe: HTTP {status} in {latency_ms} ms ({} models)",
            models.len()
        ),
    )
}
