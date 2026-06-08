use super::client::bearer_get;
use super::types::ProbeOutcome;

pub(super) async fn probe_generic_provider(
    client: &reqwest::Client,
    base_url: &str,
    credential: Option<&str>,
) -> ProbeOutcome {
    let url = base_url.trim().trim_end_matches('/').to_string();

    let _ = bearer_get(client, &url, credential).send().await;
    let started_at = std::time::Instant::now();
    let result = bearer_get(client, &url, credential).send().await;
    let latency_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;

    match result {
        Ok(response) => {
            let status = response.status().as_u16();
            (
                true,
                Some(status),
                Some(latency_ms),
                format!("Reachable: HTTP {} in {} ms", status, latency_ms),
            )
        }
        Err(err) => (false, None, None, format!("Request failed: {}", err)),
    }
}
