use super::client::provider_test_client;
use super::codex::probe_codex_provider;
use super::generic::probe_generic_provider;
use super::types::ProviderTestMessage;
use crate::theme::ProviderConfig;

pub(super) fn provider_test_credential(
    agent_name: &str,
    provider: &ProviderConfig,
) -> Option<String> {
    if agent_name == "codex" {
        provider.codex_auth_token()
    } else if provider.api_key.is_empty() {
        None
    } else {
        Some(provider.api_key.clone())
    }
}

pub(super) async fn run_provider_test_probe(
    agent_idx: usize,
    provider_idx: usize,
    agent_name: String,
    base_url: String,
    credential: Option<String>,
) -> ProviderTestMessage {
    let client = match provider_test_client() {
        Ok(client) => client,
        Err(err) => {
            return (
                agent_idx,
                provider_idx,
                false,
                None,
                None,
                format!("Failed to build HTTP client: {}", err),
            );
        }
    };

    let (success, http_status, latency, message) = if agent_name == "codex" {
        probe_codex_provider(&client, &base_url, credential.as_deref()).await
    } else {
        probe_generic_provider(&client, &base_url, credential.as_deref()).await
    };

    (
        agent_idx,
        provider_idx,
        success,
        http_status,
        latency,
        message,
    )
}
