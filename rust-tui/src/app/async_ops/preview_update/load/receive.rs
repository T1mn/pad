use crate::app::App;
use crate::log_debug;
use crate::preview_source::{PreviewRequest, PreviewUpdate};
use tokio::sync::mpsc;

impl App {
    pub fn check_preview_result(&mut self) {
        if let Some(ref mut rx) = self.preview.rx {
            match rx.try_recv() {
                Ok(update) => self.handle_preview_update_result(update),
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.finish_preview_load();
                }
            }
        }
    }

    fn handle_preview_update_result(&mut self, update: PreviewUpdate) {
        let pending = self.preview.pending_update_request.take();
        let should_skip_stale = pending
            .as_ref()
            .is_some_and(|request| request.target_key != update.target_key);
        self.finish_preview_load();

        self.apply_or_defer_preview_update(update, pending.as_ref(), should_skip_stale);

        if let Some(request) = pending {
            if should_skip_stale {
                self.trigger_async_preview_update(request);
            }
        }
    }

    fn finish_preview_load(&mut self) {
        self.preview.update_in_progress = false;
        self.preview.priority_refresh = false;
        self.preview.rx = None;
    }

    fn apply_or_defer_preview_update(
        &mut self,
        update: PreviewUpdate,
        pending: Option<&PreviewRequest>,
        should_skip_stale: bool,
    ) {
        if should_skip_stale {
            log_debug!(
                "preview.load: discard_stale loaded={} queued={}",
                update.target_key,
                pending
                    .map(|request| request.target_key.as_str())
                    .unwrap_or("")
            );
        } else if self.preview_navigation_debounce_active() {
            log_debug!(
                "preview.load: defer_result_during_navigation target={}",
                update.target_key
            );
            self.preview.deferred_preview_update = Some(update);
        } else if self.should_defer_ui_updates() {
            log_debug!("async_ops: defer preview update while in detail view");
            self.preview.deferred_preview_update = Some(update);
        } else {
            self.apply_preview_update_result(update);
        }
    }
}
