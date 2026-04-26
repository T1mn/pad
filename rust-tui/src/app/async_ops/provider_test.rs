use crate::app::App;
use tokio::sync::mpsc;

impl App {
    pub fn trigger_provider_test(&mut self, agent_idx: usize, provider_idx: usize) {
        if self.provider_test_in_progress {
            return;
        }
        let agent = match self.config.agents.get(agent_idx) {
            Some(a) => a,
            None => return,
        };
        let prov = match agent.providers.get(provider_idx) {
            Some(p) => p,
            None => return,
        };

        let agent_name = agent.name.clone();
        let base_url = prov.base_url.clone();
        let credential = if agent.name == "codex" {
            prov.codex_auth_token()
        } else if prov.api_key.is_empty() {
            None
        } else {
            Some(prov.api_key.clone())
        };

        if base_url.trim().is_empty() {
            if let Some(agent) = self.config.agents.get_mut(agent_idx) {
                if let Some(prov) = agent.providers.get_mut(provider_idx) {
                    prov.test_status = None;
                    prov.test_http_status = None;
                    prov.test_latency_ms = None;
                    prov.test_result = Some(if agent.name == "opencode" {
                        "Base URL is empty; OpenCode provider can still work if the SDK package uses non-HTTP auth or external defaults".to_string()
                    } else {
                        "Base URL is empty".to_string()
                    });
                }
            }
            self.dirty = true;
            return;
        }

        self.provider_test_in_progress = true;
        let (tx, rx) = mpsc::channel(1);
        self.provider_test_rx = Some(rx);

        tokio::spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("pad-provider-test/0.1")
                .build()
            {
                Ok(client) => client,
                Err(err) => {
                    let _ = tx
                        .send((
                            agent_idx,
                            provider_idx,
                            false,
                            None,
                            None,
                            format!("Failed to build HTTP client: {}", err),
                        ))
                        .await;
                    return;
                }
            };

            let (success, http_status, latency, message) = if agent_name == "codex" {
                probe_codex_provider(&client, &base_url, credential.as_deref()).await
            } else {
                let url = base_url.trim().trim_end_matches('/').to_string();
                let issue_request = |client: &reqwest::Client| {
                    let mut request = client.get(&url);
                    if let Some(token) =
                        credential.as_ref().filter(|token| !token.trim().is_empty())
                    {
                        request = request.bearer_auth(token);
                    }
                    request
                };

                let _ = issue_request(&client).send().await;
                let started_at = std::time::Instant::now();
                let result = issue_request(&client).send().await;
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
            };

            let _ = tx
                .send((
                    agent_idx,
                    provider_idx,
                    success,
                    http_status,
                    latency,
                    message,
                ))
                .await;
        });
    }

    pub fn check_provider_test_result(&mut self) {
        if let Some(ref mut rx) = self.provider_test_rx {
            match rx.try_recv() {
                Ok((agent_idx, prov_idx, success, http_status, latency_ms, message)) => {
                    if let Some(agent) = self.config.agents.get_mut(agent_idx) {
                        if let Some(prov) = agent.providers.get_mut(prov_idx) {
                            prov.test_status = Some(success);
                            prov.test_http_status = http_status;
                            prov.test_latency_ms = latency_ms;
                            prov.test_result = Some(message);
                        }
                    }
                    self.provider_test_in_progress = false;
                    self.provider_test_rx = None;
                    self.dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.provider_test_in_progress = false;
                    self.provider_test_rx = None;
                }
            }
        }
    }
}

async fn probe_codex_provider(
    client: &reqwest::Client,
    base_url: &str,
    credential: Option<&str>,
) -> (bool, Option<u16>, Option<u64>, String) {
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
        let mut request = client.get(&url);
        if let Some(token) = credential.filter(|token| !token.trim().is_empty()) {
            request = request.bearer_auth(token);
        }

        let response = match request.send().await {
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

        let normalized = candidate != input_base;
        let message = if normalized {
            format!(
                "Reachable for Codex relay probe via normalized API base {}: HTTP {} in {} ms ({} models)",
                candidate,
                status,
                latency_ms,
                models.len()
            )
        } else {
            format!(
                "Reachable for Codex relay probe: HTTP {} in {} ms ({} models)",
                status,
                latency_ms,
                models.len()
            )
        };
        return (true, Some(status), Some(latency_ms), message);
    }

    if last_message.is_empty() {
        last_message = "Codex relay probe failed".to_string();
    }

    (false, last_http_status, None, last_message)
}
