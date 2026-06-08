use super::types::ProviderTestMessage;
use crate::app::App;

pub(super) fn apply_empty_base_url_result(
    app: &mut App,
    agent_idx: usize,
    provider_idx: usize,
    agent_name: &str,
) {
    if let Some(agent) = app.config.agents.get_mut(agent_idx) {
        if let Some(prov) = agent.providers.get_mut(provider_idx) {
            prov.test_status = None;
            prov.test_http_status = None;
            prov.test_latency_ms = None;
            prov.test_result = Some(if agent_name == "opencode" {
                "Base URL is empty; OpenCode provider can still work if the SDK package uses non-HTTP auth or external defaults".to_string()
            } else {
                "Base URL is empty".to_string()
            });
        }
    }
    app.dirty = true;
}

pub(super) fn apply_provider_test_result(app: &mut App, result: ProviderTestMessage) {
    let (agent_idx, prov_idx, success, http_status, latency_ms, message) = result;
    if let Some(agent) = app.config.agents.get_mut(agent_idx) {
        if let Some(prov) = agent.providers.get_mut(prov_idx) {
            prov.test_status = Some(success);
            prov.test_http_status = http_status;
            prov.test_latency_ms = latency_ms;
            prov.test_result = Some(message);
        }
    }
    clear_provider_test_state(app);
    app.dirty = true;
}

pub(super) fn clear_provider_test_state(app: &mut App) {
    app.provider_test_in_progress = false;
    app.provider_test_rx = None;
}
