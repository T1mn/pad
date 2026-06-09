mod claude;
mod client;
mod codex;
mod generic;
mod probe;
mod result;
mod types;

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
