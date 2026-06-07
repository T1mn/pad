use super::super::App;
use crate::log_debug;
use crate::model::PreviewView;
use crate::theme::Theme;
use std::time::Instant;
use tokio::sync::mpsc;

impl App {
    pub fn trigger_async_preview_detail_render(&mut self, width: u16) {
        if self.preview.detail_render_in_progress {
            return;
        }
        let Some(mut request) = self.current_preview_detail_request() else {
            return;
        };
        request.width = width;
        if self
            .preview
            .detail_pending_request
            .as_ref()
            .is_some_and(|pending| pending == &request)
        {
            return;
        }
        self.preview.detail_render_in_progress = true;
        self.preview.detail_pending_request = Some(request.clone());
        let theme = Theme::by_name(&request.theme_name);
        let (tx, rx) = mpsc::channel::<crate::app::PreviewDetailCache>(1);
        self.preview.detail_render_rx = Some(rx);

        tokio::task::spawn_blocking(move || {
            let started_at = Instant::now();
            let turn = crate::model::PreviewTurn {
                question: request.question.clone(),
                answer: request.answer.clone(),
            };
            let lines =
                crate::ui::preview::render_session_detail_lines(&turn, request.width, &theme);
            let elapsed = started_at.elapsed();
            if elapsed >= std::time::Duration::from_millis(8) {
                log_debug!(
                    "preview.detail.async: render_slow target={} turn={} width={} lines={} elapsed_ms={}",
                    request.target_key,
                    request.turn_index,
                    request.width,
                    lines.len(),
                    elapsed.as_millis()
                );
            }
            let _ = tx.blocking_send(crate::app::PreviewDetailCache {
                target_key: request.target_key,
                turns: request.turns,
                turn_index: request.turn_index,
                width: request.width,
                theme_name: request.theme_name,
                question: request.question,
                answer: request.answer,
                lines,
            });
        });
    }

    pub fn check_preview_detail_result(&mut self) {
        if let Some(ref mut rx) = self.preview.detail_render_rx {
            match rx.try_recv() {
                Ok(cache) => {
                    self.store_preview_detail_cache(cache);
                    self.preview.detail_render_in_progress = false;
                    self.preview.detail_render_rx = None;
                    self.preview.detail_pending_request = None;
                    self.dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.preview.detail_render_in_progress = false;
                    self.preview.detail_render_rx = None;
                    self.preview.detail_pending_request = None;
                }
            }
        }
    }

    pub fn check_preview_detail_update(&mut self, width: u16) {
        if self.preview.view != PreviewView::SessionDetail {
            return;
        }
        let Some(selected) = self.preview.expanded_turn else {
            return;
        };
        let Some(turn) = self.preview.turns.get(selected).cloned() else {
            return;
        };
        let target_key = self.preview.pane_id.clone().unwrap_or_default();
        let theme_name = self.theme.name.to_string();
        if self
            .cached_preview_detail_for(
                &target_key,
                selected,
                width,
                &theme_name,
                &turn.question,
                &turn.answer,
            )
            .is_some()
        {
            return;
        }
        self.trigger_async_preview_detail_render(width);
    }
}
