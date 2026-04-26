use crate::app::App;
use crate::log_debug;
use crate::model::PreviewTurn;
use tokio::sync::mpsc;

impl App {
    pub fn trigger_codex_title_summary(
        &mut self,
        session_id: String,
        turns: Vec<PreviewTurn>,
        turn_count: usize,
    ) {
        if !crate::title_summary::is_enabled(&self.config.codex) {
            return;
        }

        let meta = match crate::thread_meta::load_thread_meta("codex", &session_id) {
            Ok(meta) => meta,
            Err(err) => {
                log_debug!(
                    "title_summary: failed to load thread meta session={} err={}",
                    session_id,
                    err
                );
                None
            }
        };

        if meta
            .as_ref()
            .and_then(|meta| meta.title_override.as_deref())
            .and_then(crate::sidebar::clean_title)
            .is_some()
        {
            return;
        }

        let generated_turn_count = meta.as_ref().and_then(|meta| meta.generated_turn_count);
        if !crate::title_summary::should_refresh_title(turn_count, generated_turn_count) {
            return;
        }

        let selected_turns = crate::title_summary::select_turn_window(&turns, generated_turn_count);
        if selected_turns.len() < 3 {
            return;
        }

        let request_key = format!("codex:{}:{}", session_id, turn_count);
        if !self.title_summary_in_flight.insert(request_key.clone()) {
            return;
        }

        let Some((base_url, credential, wire_api)) = self.active_codex_summary_provider() else {
            self.title_summary_in_flight.remove(&request_key);
            return;
        };

        let tx = self.ensure_title_summary_channel();
        tokio::spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(8))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("pad-title-summary/0.1")
                .build()
            {
                Ok(client) => client,
                Err(err) => {
                    let _ = tx
                        .send(crate::title_summary::TitleSummaryResult {
                            request_key,
                            session_id,
                            turn_count,
                            title: None,
                            error: Some(format!("failed to build HTTP client: {}", err)),
                        })
                        .await;
                    return;
                }
            };

            let result = crate::title_summary::request_title_summary(
                &client,
                &base_url,
                credential.as_deref(),
                wire_api,
                &selected_turns,
            )
            .await;

            let _ = tx
                .send(crate::title_summary::TitleSummaryResult {
                    request_key,
                    session_id,
                    turn_count,
                    title: result.as_ref().ok().cloned(),
                    error: result.err(),
                })
                .await;
        });
    }

    pub fn check_title_summary_result(&mut self) {
        let mut results = Vec::new();
        let mut disconnected = false;

        if let Some(ref mut rx) = self.title_summary_rx {
            loop {
                match rx.try_recv() {
                    Ok(result) => results.push(result),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        if disconnected {
            self.title_summary_rx = None;
            self.title_summary_tx = None;
            self.title_summary_in_flight.clear();
        }

        for result in results {
            self.apply_title_summary_result(result);
        }
    }

    fn ensure_title_summary_channel(
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

    fn active_codex_summary_provider(
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

    fn apply_title_summary_result(&mut self, result: crate::title_summary::TitleSummaryResult) {
        self.title_summary_in_flight.remove(&result.request_key);

        if let Some(err) = result.error {
            log_debug!(
                "title_summary: request failed session={} turns={} err={}",
                result.session_id,
                result.turn_count,
                err
            );
            return;
        }

        let Some(title) = result.title else {
            return;
        };

        let meta = match crate::thread_meta::load_thread_meta("codex", &result.session_id) {
            Ok(meta) => meta,
            Err(err) => {
                log_debug!(
                    "title_summary: failed to re-load thread meta session={} err={}",
                    result.session_id,
                    err
                );
                None
            }
        };

        if meta
            .as_ref()
            .and_then(|meta| meta.title_override.as_deref())
            .and_then(crate::sidebar::clean_title)
            .is_some()
        {
            return;
        }

        if meta
            .as_ref()
            .and_then(|meta| meta.generated_turn_count)
            .is_some_and(|existing| existing > result.turn_count)
        {
            return;
        }

        match crate::thread_meta::upsert_generated_title(
            "codex",
            &result.session_id,
            &title,
            result.turn_count,
        ) {
            Ok(()) => {
                self.invalidate_sidebar_cache();
                self.sync_sidebar_selection();
                self.invalidate_preview();
                self.dirty = true;
            }
            Err(err) => {
                log_debug!(
                    "title_summary: failed to persist generated title session={} err={}",
                    result.session_id,
                    err
                );
            }
        }
    }
}
