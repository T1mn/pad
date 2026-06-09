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

        if base_url.trim().is_empty() {
            apply_empty_base_url_result(self, agent_idx, provider_idx, &agent_name);
            return;
        }

        self.provider_test_in_progress = true;
        let (tx, rx) = mpsc::channel(1);
        self.provider_test_rx = Some(rx);

        tokio::spawn(async move {
            let result =
                run_provider_test_probe(agent_idx, provider_idx, agent_name, base_url, credential)
                    .await;
            let _ = tx.send(result).await;
        });
    }

    pub fn check_provider_test_result(&mut self) {
        if let Some(ref mut rx) = self.provider_test_rx {
            match rx.try_recv() {
                Ok(result) => apply_provider_test_result(self, result),
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => clear_provider_test_state(self),
            }
        }
    }
}
