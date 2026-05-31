use super::App;
use crate::log_debug;
use crate::model::{AgentPanel, AgentState, AgentStateSource, PreviewView};
use crate::preview_source::{self, PreviewRequest, PreviewUpdate};
use crate::scanner::scan_panels;
use crate::theme::Theme;
use std::error::Error;
use std::time::Instant;
use tokio::sync::mpsc;

mod codex_cli;
mod provider_test;
mod title_summary;

/// Async scan result channel type
pub type ScanResult = Result<Vec<AgentPanel>, Box<dyn Error + Send + Sync>>;

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

    fn panels_affecting_refresh_changed(&self, next_panels: &[AgentPanel]) -> bool {
        if self.panels.len() != next_panels.len() {
            return true;
        }

        for next in next_panels {
            let Some(current) = self
                .panels
                .iter()
                .find(|panel| panel.pane_id == next.pane_id)
            else {
                return true;
            };
            if current.session != next.session
                || current.window != next.window
                || current.window_index != next.window_index
                || current.pane != next.pane
                || current.working_dir != next.working_dir
                || current.agent_type != next.agent_type
                || current.state != next.state
                || current.state_source != next.state_source
                || current.is_active != next.is_active
                || current.transcript_path != next.transcript_path
                || current.cached_preview_turns != next.cached_preview_turns
                || current.session_cache_state != next.session_cache_state
                || current.git_info != next.git_info
                || current.pid != next.pid
                || current.last_user_prompt != next.last_user_prompt
                || current.last_assistant_message != next.last_assistant_message
                || current.agent_session_id != next.agent_session_id
                || current.has_unread_stop != next.has_unread_stop
            {
                return true;
            }
        }

        false
    }

    fn apply_scan_panels(&mut self, mut panels: Vec<AgentPanel>) {
        log_debug!("async_ops: 扫描完成，检测到 {} 个面板", panels.len());

        for panel in &mut panels {
            if let Some(existing) = self.panels.iter().find(|p| p.pane_id == panel.pane_id) {
                if existing.agent_session_id.is_some() {
                    panel.agent_session_id = existing.agent_session_id.clone();
                }
                if existing.last_user_prompt.is_some() {
                    panel.last_user_prompt = existing.last_user_prompt.clone();
                }
                if existing.last_assistant_message.is_some() {
                    panel.last_assistant_message = existing.last_assistant_message.clone();
                }
                if existing.transcript_path.is_some() {
                    panel.transcript_path = existing.transcript_path.clone();
                }
                if !existing.cached_preview_turns.is_empty() {
                    panel.cached_preview_turns = existing.cached_preview_turns.clone();
                }
                if existing.session_cache_state.is_some() {
                    panel.session_cache_state = existing.session_cache_state;
                }
                panel.has_unread_stop = existing.has_unread_stop;
                if should_preserve_hook_state(existing) {
                    panel.state = existing.state.clone();
                    panel.state_source = existing.state_source.clone();
                    panel.is_active = existing.is_active;
                }
            }
        }

        if let Err(err) = crate::session_cache::preload_panels(&mut panels) {
            log_debug!("session_cache: preload after scan failed: {}", err);
        }

        let refresh_changed = self.panels_affecting_refresh_changed(&panels);
        let sidebar_cache_empty =
            !panels.is_empty() && self.sidebar.visible_sidebar_items_cache.is_empty();
        self.panels = panels;
        let startup_sort_seeded = self.seed_startup_thread_sort_activity_once();
        if startup_sort_seeded || refresh_changed || sidebar_cache_empty {
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
        }
        if self.selected_panel().is_none() {
            self.focus_panel();
        }
        self.last_refresh = Instant::now();
        if refresh_changed {
            self.invalidate_preview();
        }
        if startup_sort_seeded || refresh_changed || sidebar_cache_empty {
            self.dirty = true;
        }
    }

    fn apply_preview_update_result(&mut self, update: PreviewUpdate) {
        let cached_detail_turn = self.preview.detail_cache.as_ref().and_then(|cache| {
            self.preview.turns.get(cache.turn_index).map(|turn| {
                (
                    cache.target_key.clone(),
                    cache.turn_index,
                    turn.clone(),
                    cache.width,
                    cache.theme_name.clone(),
                )
            })
        });
        let cached_plain_context = self.preview.plain_cache.as_ref().map(|cache| {
            (
                cache.target_key.clone(),
                cache.width,
                cache.theme_name.clone(),
                cache.content.clone(),
            )
        });
        let previous_panel_cache_state = self
            .panels
            .iter()
            .find(|panel| update.live_pane_id.as_deref() == Some(panel.pane_id.as_str()))
            .and_then(|panel| panel.session_cache_state);
        let previous_pane_id = self.preview.pane_id.clone();
        let previous_source = self.preview.source;
        let previous_view = self.preview.view;
        let previous_session_origin = self.preview.session_origin;
        let previous_session_id = self.preview.session_id.clone();
        let previous_selected_turn = self.preview.selected_turn;
        let previous_expanded_turn = self.preview.expanded_turn;
        let previous_list_scroll = self.preview.list_scroll;
        let previous_detail_scroll = self.preview.detail_scroll;
        let previous_follow_bottom = self.preview.follow_bottom;
        let previous_follow_selection = self.preview.follow_selection;
        let content_changed = self.preview.content != update.content;
        let turns_changed = self.preview.turns != update.turns;
        let should_follow_bottom = self.preview.follow_bottom
            || self.preview.pane_id.is_none()
            || self.preview.pane_id.as_deref() != Some(update.target_key.as_str());
        let same_context = self.preview.pane_id.as_deref() == Some(update.target_key.as_str())
            && self.preview.source == update.source
            && self.preview.session_origin == update.session_origin
            && self.preview.session_id == update.session_id;
        self.preview.content = update.content;
        self.preview.pane_id = Some(update.target_key.clone());
        self.preview.source = update.source;
        self.preview.session_origin = update.session_origin;
        self.preview.session_id = update.session_id.clone();
        if self.preview.source == crate::model::PreviewSource::Session && !update.turns.is_empty() {
            if !same_context {
                self.preview.selected_turn = None;
                self.preview.expanded_turn = None;
                self.preview.view = PreviewView::SessionList;
                self.preview.detail_scroll = 0;
                self.preview.list_scroll = 0;
                self.preview.follow_selection = true;
            } else {
                self.preview.selected_turn = self
                    .preview
                    .selected_turn
                    .filter(|idx| *idx < update.turns.len());
                self.preview.expanded_turn = self
                    .preview
                    .expanded_turn
                    .filter(|idx| *idx < update.turns.len());
                self.preview.view = if self.preview.expanded_turn.is_some() {
                    PreviewView::SessionDetail
                } else {
                    PreviewView::SessionList
                };
            }
            self.preview.turns = update.turns.clone();
            self.preview.follow_bottom = false;
        } else {
            self.preview.turns = Default::default();
            self.preview.session_origin = None;
            self.preview.session_id = None;
            self.preview.selected_turn = None;
            self.preview.expanded_turn = None;
            self.preview.view = PreviewView::Plain;
            self.preview.list_scroll = 0;
            self.preview.detail_scroll = 0;
            self.preview.follow_bottom = should_follow_bottom;
            self.preview.follow_selection = true;
        }

        let preserve_detail_cache = cached_detail_turn.as_ref().is_some_and(
            |(target_key, turn_index, previous_turn, width, theme_name)| {
                self.cached_preview_detail_for(
                    target_key,
                    *turn_index,
                    *width,
                    theme_name,
                    &previous_turn.question,
                    &previous_turn.answer,
                )
                .is_some()
            },
        );
        if !preserve_detail_cache {
            self.preview.detail_cache = None;
            self.preview.detail_lru.clear();
            self.preview.detail_render_in_progress = false;
            self.preview.detail_render_rx = None;
            self.preview.detail_pending_request = None;
        }

        let preserve_plain_cache = cached_plain_context.as_ref().is_some_and(
            |(target_key, _width, theme_name, content)| {
                self.preview.pane_id.as_deref() == Some(target_key.as_str())
                    && self.preview.source == crate::model::PreviewSource::Tmux
                    && self.preview.view == PreviewView::Plain
                    && self.theme.name == theme_name
                    && self.preview.content == *content
            },
        );
        if !preserve_plain_cache {
            self.preview.plain_cache = None;
        }

        let mut panel_cache_state_changed = false;
        if update.source == crate::model::PreviewSource::Session && !update.turns.is_empty() {
            let previous_updated_at = self
                .preview
                .thread_preview_cache
                .get(&update.target_key)
                .and_then(|entry| entry.updated_at);
            self.preview.thread_preview_cache.insert(
                update.target_key.clone(),
                crate::app::ThreadPreviewCacheEntry {
                    turns: update.turns.clone(),
                    session_cache_state: update.session_cache_state,
                    transcript_path: update.transcript_path.clone(),
                    session_id: update.session_id.clone(),
                    updated_at: update.updated_at,
                    cached_at: crate::app::unix_now_ts(),
                },
            );
            let preview_cache_pruned = self.prune_thread_preview_cache();
            if update.updated_at != previous_updated_at || preview_cache_pruned {
                self.invalidate_sidebar_cache();
            }
        }
        if let Some(panel) = update.live_pane_id.as_deref().and_then(|pane_id| {
            self.panels
                .iter_mut()
                .find(|panel| panel.pane_id == pane_id)
        }) {
            let should_persist_panel_session =
                update.session_origin != Some(crate::model::PreviewSessionOrigin::App);
            if should_persist_panel_session {
                if let Some(transcript_path) = update.transcript_path.clone() {
                    panel.transcript_path = Some(transcript_path);
                }
            }
            if self.preview.source == crate::model::PreviewSource::Session
                && !update.turns.is_empty()
                && should_persist_panel_session
            {
                panel.cached_preview_turns = update.turns.clone();
                panel.last_user_prompt = update.turns.first().map(|turn| turn.question.clone());
                panel.last_assistant_message =
                    update.turns.first().and_then(|turn| turn.answer.clone());
                if let Some(state) = update.session_cache_state {
                    panel.session_cache_state = Some(state);
                }
            }
            if should_persist_panel_session {
                if let Some(session_id) = update.session_id.clone() {
                    panel.agent_session_id = Some(session_id);
                }
            }
            if should_persist_panel_session {
                panel_cache_state_changed = previous_panel_cache_state != panel.session_cache_state;
            }
        }

        self.preview.last_preview_update = Instant::now();
        if previous_pane_id != self.preview.pane_id
            || previous_source != self.preview.source
            || previous_view != self.preview.view
            || previous_session_origin != self.preview.session_origin
            || previous_session_id != self.preview.session_id
            || content_changed
            || turns_changed
            || previous_selected_turn != self.preview.selected_turn
            || previous_expanded_turn != self.preview.expanded_turn
            || previous_list_scroll != self.preview.list_scroll
            || previous_detail_scroll != self.preview.detail_scroll
            || previous_follow_bottom != self.preview.follow_bottom
            || previous_follow_selection != self.preview.follow_selection
            || panel_cache_state_changed
        {
            self.dirty = true;
        }
    }

    pub fn flush_deferred_ui_updates(&mut self) {
        if self.should_defer_ui_updates() {
            return;
        }

        if let Some(panels) = self.deferred_scan_result.take() {
            self.apply_scan_panels(panels);
        }

        if let Some(update) = self.preview.deferred_preview_update.take() {
            if self.preview_navigation_debounce_active() {
                self.preview.deferred_preview_update = Some(update);
            } else if self.preview_update_matches_current_selection(&update) {
                self.apply_preview_update_result(update);
            } else {
                log_debug!(
                    "preview.load: discard_deferred_stale target={}",
                    update.target_key
                );
            }
        }

        if !self.deferred_hook_events.is_empty() {
            let events = std::mem::take(&mut self.deferred_hook_events);
            for event in events {
                self.apply_hook_event(event);
            }
        }
    }

    pub fn trigger_async_scan(&mut self) {
        if self.scan_in_progress {
            return;
        }

        self.scan_in_progress = true;
        let (tx, rx) = mpsc::channel::<ScanResult>(1);
        self.scan_rx = Some(rx);

        tokio::task::spawn_blocking(move || {
            let result = scan_panels();
            let _ = tx.blocking_send(result);
        });
    }

    pub fn check_scan_result(&mut self) {
        if let Some(ref mut rx) = self.scan_rx {
            match rx.try_recv() {
                Ok(Ok(panels)) => {
                    if self.should_defer_ui_updates() {
                        log_debug!("async_ops: defer scan result while in detail view");
                        self.deferred_scan_result = Some(panels);
                    } else {
                        self.apply_scan_panels(panels);
                    }
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                }
                Ok(Err(e)) => {
                    log_debug!("async_ops: 扫描失败: {}", e);
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    log_debug!("async_ops: 扫描 channel 断开");
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                }
            }
        }
    }

    pub fn schedule_delayed_scan(&mut self, delay_ms: u64) {
        self.delayed_scan_at = Some(Instant::now() + std::time::Duration::from_millis(delay_ms));
    }

    pub fn check_delayed_scan(&mut self) {
        if let Some(at) = self.delayed_scan_at {
            if Instant::now() >= at {
                self.delayed_scan_at = None;
                if !self.scan_in_progress {
                    self.trigger_async_scan();
                }
            }
        }
    }

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

    pub fn check_preview_result(&mut self) {
        if let Some(ref mut rx) = self.preview.rx {
            match rx.try_recv() {
                Ok(update) => {
                    let pending = self.preview.pending_update_request.take();
                    let should_skip_stale = pending
                        .as_ref()
                        .is_some_and(|request| request.target_key != update.target_key);
                    self.preview.update_in_progress = false;
                    self.preview.priority_refresh = false;
                    self.preview.rx = None;

                    if should_skip_stale {
                        log_debug!(
                            "preview.load: discard_stale loaded={} queued={}",
                            update.target_key,
                            pending
                                .as_ref()
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

                    if let Some(request) = pending {
                        if should_skip_stale {
                            self.trigger_async_preview_update(request);
                        }
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.preview.update_in_progress = false;
                    self.preview.priority_refresh = false;
                    self.preview.rx = None;
                }
            }
        }
    }

    pub fn check_preview_update(&mut self) {
        if self.scan_in_progress && !self.preview.priority_refresh {
            return;
        }

        if let Some(until) = self.preview.navigation_debounce_until {
            if Instant::now() < until {
                return;
            }
            self.preview.navigation_debounce_until = None;
            self.invalidate_preview();
        }

        if self.should_pause_preview_refresh() {
            return;
        }

        let request = self.selected_preview_thread().map(|thread| PreviewRequest {
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
        });

        if let Some(request) = request {
            if !self.preview.priority_refresh {
                let refresh_ms = preview_source::preview_refresh_interval_ms_for_request(&request);
                if self.preview.last_preview_update.elapsed()
                    < std::time::Duration::from_millis(refresh_ms)
                {
                    return;
                }
            }
            self.trigger_async_preview_update(request);
        }
    }

    fn preview_update_matches_current_selection(&mut self, update: &PreviewUpdate) -> bool {
        if let Some(selected_key) = self.selected_preview_thread().map(|thread| thread.key) {
            return selected_key == update.target_key;
        }

        self.preview.pane_id.as_deref() == Some(update.target_key.as_str())
    }
}

fn should_preserve_hook_state(panel: &AgentPanel) -> bool {
    panel.state_source == AgentStateSource::Hook && matches!(panel.state, AgentState::Busy)
}

#[cfg(test)]
mod async_ops_tests;
