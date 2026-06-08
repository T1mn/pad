use crate::app::App;
use crate::log_debug;
use crate::preview_source::{self, PreviewRequest, PreviewUpdate};
use std::time::Instant;
use tokio::sync::mpsc;

impl App {
    pub fn trigger_async_preview_update(&mut self, request: PreviewRequest) {
        if self.preview.update_in_progress {
            log_debug!(
                "preview.load: queue_latest target={} previous_pending={}",
                request.target_key,
                self.preview.pending_update_request.is_some()
            );
            self.preview.pending_update_request = Some(request);
            return;
        }

        self.preview.update_in_progress = true;
        self.preview.priority_refresh = false;
        let locale = self.locale;
        let preview_mode = self.config.preview.mode.clone();
        let (tx, rx) = mpsc::channel::<PreviewUpdate>(1);
        self.preview.rx = Some(rx);

        tokio::task::spawn_blocking(move || {
            let started_at = Instant::now();
            let update = preview_source::load_preview(&request, &preview_mode, locale);
            let elapsed = started_at.elapsed();
            if elapsed >= std::time::Duration::from_millis(20) {
                log_debug!(
                    "preview.load: slow target={} source_hint={:?} elapsed_ms={}",
                    request.target_key,
                    request.session_origin,
                    elapsed.as_millis()
                );
            }
            let _ = tx.blocking_send(update);
        });
    }
}
