mod receive {
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
}
mod tick {
    use crate::app::App;
    use crate::preview_source::{self, PreviewRequest};
    use std::time::{Duration, Instant};

    impl App {
        pub fn check_preview_update(&mut self) {
            if self.scan_in_progress && !self.preview.priority_refresh {
                return;
            }

            if !self.preview_navigation_debounce_elapsed() {
                return;
            }

            if self.should_pause_preview_refresh() {
                return;
            }

            let Some(request) = self.selected_preview_request() else {
                return;
            };

            if !self.preview.priority_refresh && self.preview_refresh_throttled(&request) {
                return;
            }
            self.trigger_async_preview_update(request);
        }

        fn preview_navigation_debounce_elapsed(&mut self) -> bool {
            let Some(until) = self.preview.navigation_debounce_until else {
                return true;
            };
            if Instant::now() < until {
                return false;
            }
            self.preview.navigation_debounce_until = None;
            self.invalidate_preview();
            true
        }

        fn selected_preview_request(&mut self) -> Option<PreviewRequest> {
            self.selected_preview_thread().map(|thread| PreviewRequest {
                target_key: thread.key.clone(),
                live_pane_id: thread.live_pane_id.clone(),
                agent_type: thread.agent_type.clone(),
                working_dir: thread.working_dir.clone(),
                state: thread.state.clone(),
                transcript_path: thread.transcript_path.clone(),
                cached_preview_turns: thread.cached_preview_turns.clone(),
                session_cache_state: thread.session_cache_state,
                agent_session_id: thread.session_id.clone(),
                session_origin: thread.preview_origin(),
                persist_resolved_session: thread.is_live(),
                known_updated_at: Some(thread.updated_at),
            })
        }

        fn preview_refresh_throttled(&self, request: &PreviewRequest) -> bool {
            let refresh_ms = preview_source::preview_refresh_interval_ms_for_request(request);
            self.preview.last_preview_update.elapsed() < Duration::from_millis(refresh_ms)
        }
    }
}
mod trigger {
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
}
