pub(crate) mod claude;
mod client {
    pub(super) fn provider_test_client() -> Result<reqwest::Client, reqwest::Error> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(45))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("pad-provider-test/0.1")
            .build()
    }

    pub(super) fn bearer_get<'a>(
        client: &'a reqwest::Client,
        url: &'a str,
        credential: Option<&str>,
    ) -> reqwest::RequestBuilder {
        let mut request = client.get(url);
        if let Some(token) = credential.filter(|token| !token.trim().is_empty()) {
            request = request.bearer_auth(token);
        }
        request
    }

    pub(super) fn bearer_post_json<'a>(
        client: &'a reqwest::Client,
        url: &'a str,
        credential: Option<&str>,
        payload: &'a serde_json::Value,
    ) -> reqwest::RequestBuilder {
        let mut request = client
            .post(url)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(payload);
        if let Some(token) = credential.filter(|token| !token.trim().is_empty()) {
            request = request.bearer_auth(token);
        }
        request
    }

    pub(super) fn claude_post_json<'a>(
        client: &'a reqwest::Client,
        url: &'a str,
        credential: Option<&str>,
        payload: &'a serde_json::Value,
    ) -> reqwest::RequestBuilder {
        let mut request = client
            .post(url)
            .header("anthropic-version", "2023-06-01")
            .json(payload);
        if let Some(token) = credential.filter(|token| !token.trim().is_empty()) {
            request = request.header("x-api-key", token).bearer_auth(token);
        }
        request
    }
}
mod codex;
mod generic {
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
}
mod probe {
    use super::claude::probe_claude_provider;
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
        default_model: String,
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

        let (success, http_status, latency, message) = match agent_name.as_str() {
            "codex" => probe_codex_provider(&client, &base_url, credential.as_deref()).await,
            "claude" => {
                probe_claude_provider(&client, &base_url, credential.as_deref(), &default_model)
                    .await
            }
            _ => probe_generic_provider(&client, &base_url, credential.as_deref()).await,
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
}
mod result {
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
        app.save_config();
    }
}
mod types {
    pub(crate) type ProviderTestResult = (usize, usize, bool, Option<u16>, Option<u64>, String);
    pub(super) type ProviderTestMessage = ProviderTestResult;
    pub(super) type ProbeOutcome = (bool, Option<u16>, Option<u64>, String);
}

use crate::app::App;
use probe::{provider_test_credential, run_provider_test_probe};
use result::{apply_empty_base_url_result, apply_provider_test_result, clear_provider_test_state};
use tokio::sync::mpsc;
pub(crate) use types::ProviderTestResult;

type ProviderTestJob = (usize, String, Option<String>, String);
type ProviderTestJobs = (usize, String, Vec<ProviderTestJob>);

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
        let credential = provider_test_credential(&agent.name, prov);
        let default_model = agent.default_model.clone();

        if base_url.trim().is_empty() {
            apply_empty_base_url_result(self, agent_idx, provider_idx, &agent_name);
            return;
        }

        let (tx, rx) = mpsc::channel(1);
        self.provider_test_in_progress = true;
        self.provider_test_pending_count = 1;
        self.provider_test_sort_agent_on_complete = None;
        self.provider_test_rx = Some(rx);

        tokio::spawn(async move {
            let result = run_provider_test_probe(
                agent_idx,
                provider_idx,
                agent_name,
                base_url,
                credential,
                default_model,
            )
            .await;
            let _ = tx.send(result).await;
        });
    }

    pub fn trigger_provider_batch_test_for_agent(&mut self, target_agent_name: &str) {
        if self.provider_test_in_progress {
            self.show_action_toast("Relay 批量测试未启动", "已有 provider 测试正在进行");
            return;
        }

        let Some((agent_idx, agent_name, probes)) = provider_test_jobs(self, target_agent_name)
        else {
            self.show_action_toast("Relay 批量测试未启动", "没有可测试的 provider");
            return;
        };

        let mut immediate_empty = Vec::new();
        let mut async_jobs = Vec::new();
        for (provider_idx, base_url, credential, default_model) in probes {
            if base_url.trim().is_empty() {
                immediate_empty.push(provider_idx);
            } else {
                async_jobs.push((provider_idx, base_url, credential, default_model));
            }
        }

        for provider_idx in immediate_empty {
            apply_empty_base_url_result(self, agent_idx, provider_idx, &agent_name);
        }

        if async_jobs.is_empty() {
            self.show_action_toast("Relay 批量测试已完成", "没有需要发起请求的 provider");
            return;
        }

        let async_count = async_jobs.len();
        for (provider_idx, _, _, _) in &async_jobs {
            mark_provider_test_started(self, agent_idx, *provider_idx);
        }
        self.show_action_toast(
            "Relay 批量测试已启动",
            &format!("正在测试 {agent_name} 的 {async_count} 个 provider，结果会后台返回"),
        );

        let (tx, rx) = mpsc::channel(async_jobs.len().max(1));
        self.provider_test_in_progress = true;
        self.provider_test_pending_count = async_jobs.len();
        self.provider_test_sort_agent_on_complete = Some(agent_idx);
        self.provider_test_rx = Some(rx);

        for (provider_idx, base_url, credential, default_model) in async_jobs {
            let tx = tx.clone();
            let agent_name = agent_name.clone();
            tokio::spawn(async move {
                let result = run_provider_test_probe(
                    agent_idx,
                    provider_idx,
                    agent_name,
                    base_url,
                    credential,
                    default_model,
                )
                .await;
                let _ = tx.send(result).await;
            });
        }
    }

    pub fn check_provider_test_result(&mut self) {
        loop {
            let Some(ref mut rx) = self.provider_test_rx else {
                return;
            };
            match rx.try_recv() {
                Ok(result) => apply_provider_test_result(self, result),
                Err(mpsc::error::TryRecvError::Empty) => return,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    if self.provider_test_pending_count == 0 {
                        clear_provider_test_state(self);
                    }
                    return;
                }
            }
        }
    }
}

fn provider_test_jobs(app: &App, target_agent_name: &str) -> Option<ProviderTestJobs> {
    let agent_idx = app
        .config
        .agents
        .iter()
        .position(|agent| agent.name == target_agent_name)?;
    let agent = app.config.agents.get(agent_idx)?;
    let jobs = agent
        .providers
        .iter()
        .enumerate()
        .map(|(provider_idx, provider)| {
            (
                provider_idx,
                provider.base_url.clone(),
                provider_test_credential(&agent.name, provider),
                agent.default_model.clone(),
            )
        })
        .collect::<Vec<_>>();

    (!jobs.is_empty()).then_some((agent_idx, agent.name.clone(), jobs))
}

fn mark_provider_test_started(app: &mut App, agent_idx: usize, provider_idx: usize) {
    let agent_name = agent_name_for_index(app, agent_idx);
    if let Some(provider) = app
        .config
        .agents
        .get_mut(agent_idx)
        .and_then(|agent| agent.providers.get_mut(provider_idx))
    {
        provider.test_status = None;
        provider.test_http_status = None;
        provider.test_latency_ms = None;
        provider.test_result = Some(format!("Testing real {agent_name} chat..."));
    }
    app.dirty = true;
}

fn agent_name_for_index(app: &App, agent_idx: usize) -> String {
    app.config
        .agents
        .get(agent_idx)
        .map(|agent| agent.name.clone())
        .unwrap_or_else(|| "provider".to_string())
}
