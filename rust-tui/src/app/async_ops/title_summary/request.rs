use crate::app::App;
use crate::log_debug;
use crate::model::PreviewTurn;

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
}
