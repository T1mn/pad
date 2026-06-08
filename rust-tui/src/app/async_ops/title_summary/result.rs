use crate::app::App;
use crate::log_debug;
use tokio::sync::mpsc;

impl App {
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

    pub(super) fn apply_title_summary_result(
        &mut self,
        result: crate::title_summary::TitleSummaryResult,
    ) {
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
