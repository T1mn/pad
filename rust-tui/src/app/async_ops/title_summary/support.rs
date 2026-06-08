use crate::app::App;
use crate::log_debug;
use tokio::sync::mpsc;

impl App {
    pub(super) fn ensure_title_summary_channel(
        &mut self,
    ) -> mpsc::Sender<crate::title_summary::TitleSummaryResult> {
        if let Some(tx) = self.title_summary_tx.as_ref() {
            return tx.clone();
        }

        let (tx, rx) = mpsc::channel(16);
        self.title_summary_tx = Some(tx.clone());
        self.title_summary_rx = Some(rx);
        tx
    }

    pub(super) fn active_codex_summary_provider(
        &self,
    ) -> Option<(String, Option<String>, crate::title_summary::SummaryWireApi)> {
        let agent = self
            .config
            .agents
            .iter()
            .find(|agent| agent.name == "codex")?;
        let provider = agent.active()?;
        let base_url = provider.codex_base_url();
        if base_url.trim().is_empty() {
            log_debug!("title_summary: skip because active codex provider base_url is empty");
            return None;
        }

        let credential = provider.codex_auth_token();
        if credential
            .as_deref()
            .map(str::trim)
            .is_none_or(|token| token.is_empty())
        {
            log_debug!("title_summary: skip because active codex provider credential is empty");
            return None;
        }

        Some((
            base_url,
            credential,
            crate::title_summary::SummaryWireApi::from_config(provider.codex_wire_api()),
        ))
    }
}
