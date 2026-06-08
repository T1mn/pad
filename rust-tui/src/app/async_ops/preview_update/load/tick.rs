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
