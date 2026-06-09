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
    if app.provider_test_pending_count > 0 {
        app.provider_test_pending_count -= 1;
    }
    if app.provider_test_pending_count == 0 {
        if let Some(agent_idx) = app.provider_test_sort_agent_on_complete {
            sort_providers_after_batch(app, agent_idx);
        }
        clear_provider_test_state(app);
    }
    app.dirty = true;
}

pub(super) fn clear_provider_test_state(app: &mut App) {
    app.provider_test_in_progress = false;
    app.provider_test_pending_count = 0;
    app.provider_test_sort_agent_on_complete = None;
    app.provider_test_rx = None;
}

fn sort_providers_after_batch(app: &mut App, agent_idx: usize) {
    let Some(agent) = app.config.agents.get_mut(agent_idx) else {
        return;
    };
    if agent.providers.len() < 2 {
        return;
    }

    let old_active = agent.active_provider;
    let old_selected =
        (app.relay_selected_agent == agent_idx).then_some(app.relay_selected_provider);
    let provider_count = agent.providers.len();
    let mut indexed = agent.providers.drain(..).enumerate().collect::<Vec<_>>();
    indexed.sort_by_key(|(old_idx, provider)| {
        let success_rank = if provider.test_status == Some(true) {
            0
        } else {
            1
        };
        let latency = provider.test_latency_ms.unwrap_or(u64::MAX);
        (success_rank, latency, *old_idx)
    });

    agent.active_provider = old_active.and_then(|active_idx| {
        indexed
            .iter()
            .position(|(old_idx, _)| *old_idx == active_idx)
    });
    app.relay_selected_provider = old_selected
        .and_then(|selected_idx| {
            indexed
                .iter()
                .position(|(old_idx, _)| *old_idx == selected_idx)
        })
        .unwrap_or(app.relay_selected_provider)
        .min(provider_count.saturating_sub(1));
    agent.providers = indexed.into_iter().map(|(_, provider)| provider).collect();
    app.config.save();
}
