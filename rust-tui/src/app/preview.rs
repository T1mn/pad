use super::{
    App, PreviewDetailCache, PreviewDetailRenderRequest, PreviewMouseSelection,
    THREAD_PREVIEW_CACHE_MAX_ENTRIES,
};
use crate::model::{AgentState, PreviewSource, PreviewView};
use std::collections::HashSet;
use std::time::{Duration, Instant};

impl App {
    pub fn clear_preview_render_caches(&mut self) {
        self.preview_detail_cache = None;
        self.preview_detail_lru.clear();
        self.preview_detail_render_in_progress = false;
        self.preview_detail_render_rx = None;
        self.preview_detail_pending_request = None;
        self.preview_plain_cache = None;
    }

    pub fn current_preview_detail_request(&self) -> Option<PreviewDetailRenderRequest> {
        let selected = self.preview_expanded_turn?;
        let turn = self.preview_turns.get(selected)?;
        Some(PreviewDetailRenderRequest {
            target_key: self.preview_pane_id.clone().unwrap_or_default(),
            turn_index: selected,
            width: 0,
            theme_name: self.theme.name.to_string(),
            question: turn.question.clone(),
            answer: turn.answer.clone(),
        })
    }

    pub fn cached_preview_detail_for(
        &mut self,
        target_key: &str,
        turn_index: usize,
        width: u16,
        theme_name: &str,
        question: &str,
        answer: &Option<String>,
    ) -> Option<PreviewDetailCache> {
        if let Some(cache) = self.preview_detail_cache.as_ref().filter(|cache| {
            cache.target_key == target_key
                && cache.turn_index == turn_index
                && cache.width == width
                && cache.theme_name == theme_name
                && cache.question == question
                && cache.answer == *answer
        }) {
            return Some(cache.clone());
        }

        if let Some(idx) = self.preview_detail_lru.iter().position(|cache| {
            cache.target_key == target_key
                && cache.turn_index == turn_index
                && cache.width == width
                && cache.theme_name == theme_name
                && cache.question == question
                && cache.answer == *answer
        }) {
            let cache = self.preview_detail_lru.remove(idx);
            self.preview_detail_lru.insert(0, cache.clone());
            self.preview_detail_cache = Some(cache.clone());
            return Some(cache);
        }

        None
    }

    pub fn store_preview_detail_cache(&mut self, cache: PreviewDetailCache) {
        self.preview_detail_lru.retain(|existing| {
            !(existing.target_key == cache.target_key
                && existing.turn_index == cache.turn_index
                && existing.width == cache.width
                && existing.theme_name == cache.theme_name
                && existing.question == cache.question
                && existing.answer == cache.answer)
        });
        self.preview_detail_lru.insert(0, cache.clone());
        self.preview_detail_lru.truncate(6);
        self.preview_detail_cache = Some(cache);
    }

    pub fn invalidate_preview(&mut self) {
        self.last_preview_update = Instant::now() - Duration::from_secs(1);
        self.preview_priority_refresh = true;
        self.preview_plain_cache = None;
    }

    pub(crate) fn prune_thread_preview_cache(&mut self) -> bool {
        if self.thread_preview_cache.len() <= THREAD_PREVIEW_CACHE_MAX_ENTRIES {
            return false;
        }

        let mut keys_by_freshness = self
            .thread_preview_cache
            .iter()
            .map(|(key, entry)| {
                (
                    key.clone(),
                    entry.updated_at.unwrap_or(entry.cached_at),
                    entry.cached_at,
                )
            })
            .collect::<Vec<_>>();
        keys_by_freshness.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| left.0.cmp(&right.0))
        });

        let keep = keys_by_freshness
            .into_iter()
            .take(THREAD_PREVIEW_CACHE_MAX_ENTRIES)
            .map(|item| item.0)
            .collect::<HashSet<_>>();
        let before = self.thread_preview_cache.len();
        self.thread_preview_cache
            .retain(|key, _| keep.contains(key));
        self.thread_preview_cache.len() != before
    }

    pub fn preview_is_focused(&self) -> bool {
        self.preview_focus == super::state::FocusTarget::Preview && !self.show_tree
    }

    pub fn toggle_preview_focus(&mut self) -> bool {
        if self.show_tree || self.selected_preview_thread().is_none() {
            return false;
        }
        self.preview_focus = match self.preview_focus {
            super::state::FocusTarget::Panel => super::state::FocusTarget::Preview,
            super::state::FocusTarget::Preview => super::state::FocusTarget::Panel,
        };
        self.clear_unread_stop_for_selected_panel();
        self.dirty = true;
        true
    }

    pub fn focus_panel(&mut self) {
        if self.preview_focus != super::state::FocusTarget::Panel {
            self.preview_focus = super::state::FocusTarget::Panel;
        }
        self.clear_unread_stop_for_selected_panel();
        self.dirty = true;
    }

    pub fn focus_preview(&mut self) -> bool {
        if self.show_tree || self.selected_preview_thread().is_none() {
            return false;
        }
        if self.preview_focus != super::state::FocusTarget::Preview {
            self.preview_focus = super::state::FocusTarget::Preview;
        }
        self.dirty = true;
        true
    }

    pub fn has_session_preview_turns(&self) -> bool {
        self.preview_source == PreviewSource::Session && !self.preview_turns.is_empty()
    }

    pub fn should_defer_ui_updates(&self) -> bool {
        self.preview_view == PreviewView::SessionDetail
    }

    pub fn note_panel_tab(&mut self) {
        self.last_panel_tab_at = Some(Instant::now());
    }

    pub fn recent_panel_tab_within(&self, window: Duration) -> bool {
        self.last_panel_tab_at
            .map(|instant| instant.elapsed() <= window)
            .unwrap_or(false)
    }

    pub fn clear_panel_tab(&mut self) {
        self.last_panel_tab_at = None;
    }

    pub(crate) fn preview_uses_list_scroll(&self) -> bool {
        self.has_session_preview_turns() && self.preview_view == PreviewView::SessionList
    }

    pub(crate) fn preview_uses_detail_scroll(&self) -> bool {
        self.has_session_preview_turns() && self.preview_view == PreviewView::SessionDetail
    }

    fn flush_deferred_updates_on_preview_exit(&mut self) {
        if self.should_defer_ui_updates() {
            return;
        }
        self.last_preview_update = Instant::now() - Duration::from_secs(1);
    }

    pub fn open_latest_preview_turn(&mut self) -> bool {
        if self.show_tree {
            return false;
        }

        let Some(thread) = self.selected_preview_thread() else {
            return false;
        };

        let same_session_preview = self.preview_source == PreviewSource::Session
            && self.preview_pane_id.as_deref() == Some(thread.key.as_str())
            && !self.preview_turns.is_empty();

        if !same_session_preview {
            self.preview_turns = thread.cached_preview_turns.clone();
            self.preview_pane_id = Some(thread.key.clone());
            self.preview_session_origin = thread.preview_origin();
            self.preview_session_id = thread.session_id.clone();
            if !self.preview_turns.is_empty() {
                self.preview_source = PreviewSource::Session;
            }
        }

        if !self.has_session_preview_turns() {
            return false;
        }

        self.preview_focus = super::state::FocusTarget::Preview;
        self.preview_selected_turn = Some(0);
        self.preview_expanded_turn = Some(0);
        self.preview_view = PreviewView::SessionDetail;
        self.preview_detail_scroll = 0;
        self.preview_list_scroll = 0;
        self.preview_follow_bottom = false;
        self.preview_follow_selection = true;
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }

    pub fn should_pause_preview_refresh(&self) -> bool {
        self.preview_source == PreviewSource::Session
            && self.preview_view == PreviewView::SessionDetail
    }

    pub fn should_pause_busy_animations(&self) -> bool {
        self.preview_view == PreviewView::SessionDetail
    }

    pub fn should_tick_busy_animation(&self) -> bool {
        !self.should_pause_busy_animations()
            && (self
                .panels
                .iter()
                .any(|panel| matches!(panel.state, AgentState::Busy))
                || self
                    .app_thread_activity
                    .values()
                    .any(|thread| matches!(thread.state, AgentState::Busy)))
            && self.last_busy_animation_tick.elapsed() >= self.busy_animation_interval()
    }

    pub fn select_next_preview_turn(&mut self) -> bool {
        if !self.has_session_preview_turns() {
            return false;
        }
        let max = self.preview_turns.len().saturating_sub(1);
        let next = match self.preview_selected_turn {
            Some(idx) => (idx + 1).min(max),
            None => 0,
        };
        self.preview_selected_turn = Some(next);
        if self.preview_view == PreviewView::SessionDetail {
            self.preview_expanded_turn = Some(next);
            self.preview_detail_scroll = 0;
        }
        self.preview_follow_bottom = false;
        self.preview_follow_selection = true;
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }

    pub fn select_previous_preview_turn(&mut self) -> bool {
        if !self.has_session_preview_turns() {
            return false;
        }
        let prev = match self.preview_selected_turn {
            Some(idx) => idx.saturating_sub(1),
            None => 0,
        };
        self.preview_selected_turn = Some(prev);
        if self.preview_view == PreviewView::SessionDetail {
            self.preview_expanded_turn = Some(prev);
            self.preview_detail_scroll = 0;
        }
        self.preview_follow_bottom = false;
        self.preview_follow_selection = true;
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }

    pub fn select_preview_turn(&mut self, index: usize) -> bool {
        if !self.has_session_preview_turns() || index >= self.preview_turns.len() {
            return false;
        }
        self.preview_focus = super::state::FocusTarget::Preview;
        self.preview_selected_turn = Some(index);
        self.preview_expanded_turn = None;
        self.preview_view = PreviewView::SessionList;
        self.preview_detail_scroll = 0;
        self.preview_follow_bottom = false;
        self.preview_follow_selection = true;
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }

    pub fn step_back_preview_focus(&mut self) -> bool {
        if self.preview_view == PreviewView::SessionDetail {
            self.preview_view = PreviewView::SessionList;
            self.preview_expanded_turn = None;
            self.preview_detail_scroll = 0;
            self.preview_follow_selection = true;
            self.flush_deferred_updates_on_preview_exit();
            self.clear_preview_render_caches();
            self.dirty = true;
            return true;
        }
        if self.preview_selected_turn.is_some() {
            self.preview_selected_turn = None;
            self.preview_view = if self.has_session_preview_turns() {
                PreviewView::SessionList
            } else {
                PreviewView::Plain
            };
            self.preview_follow_selection = false;
            self.clear_preview_render_caches();
            self.dirty = true;
            return true;
        }
        if self.preview_is_focused() {
            self.preview_focus = super::state::FocusTarget::Panel;
            self.clear_unread_stop_for_selected_panel();
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn toggle_preview_turn_expanded(&mut self) -> bool {
        if !self.has_session_preview_turns() {
            return false;
        }
        let Some(selected) = self.preview_selected_turn else {
            return false;
        };
        if self.preview_view == PreviewView::SessionDetail
            && self.preview_expanded_turn == Some(selected)
        {
            self.preview_view = PreviewView::SessionList;
            self.preview_expanded_turn = None;
            self.flush_deferred_updates_on_preview_exit();
        } else {
            self.preview_view = PreviewView::SessionDetail;
            self.preview_expanded_turn = Some(selected);
        }
        self.preview_detail_scroll = 0;
        self.preview_follow_bottom = false;
        self.preview_follow_selection = true;
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }

    pub fn scroll_preview_by(&mut self, delta: i32) {
        if self.preview_uses_list_scroll() {
            self.preview_follow_selection = false;
            if delta >= 0 {
                self.preview_list_scroll = self.preview_list_scroll.saturating_add(delta as u16);
            } else {
                self.preview_list_scroll = self.preview_list_scroll.saturating_sub((-delta) as u16);
            }
        } else if self.preview_uses_detail_scroll() {
            if delta >= 0 {
                self.preview_detail_scroll =
                    self.preview_detail_scroll.saturating_add(delta as u16);
            } else {
                self.preview_detail_scroll =
                    self.preview_detail_scroll.saturating_sub((-delta) as u16);
            }
        } else {
            self.preview_follow_bottom = false;
            if delta >= 0 {
                self.preview_scroll = self.preview_scroll.saturating_add(delta as u16);
            } else {
                self.preview_scroll = self.preview_scroll.saturating_sub((-delta) as u16);
            }
        }
        self.dirty = true;
    }

    pub fn scroll_preview_to_top(&mut self) {
        if self.preview_uses_list_scroll() {
            self.preview_list_scroll = 0;
            self.preview_follow_selection = false;
        } else if self.preview_uses_detail_scroll() {
            self.preview_detail_scroll = 0;
        } else {
            self.preview_scroll = 0;
            self.preview_follow_bottom = false;
        }
        self.dirty = true;
    }

    pub fn scroll_preview_to_bottom(&mut self) {
        if self.preview_uses_list_scroll() {
            self.preview_list_scroll = u16::MAX;
            self.preview_follow_selection = false;
        } else if self.preview_uses_detail_scroll() {
            self.preview_detail_scroll = u16::MAX;
        } else {
            self.preview_follow_bottom = true;
        }
        self.dirty = true;
    }

    pub fn desired_tick_rate(&self) -> Duration {
        if self.preview_view == PreviewView::SessionDetail {
            Duration::from_millis(90)
        } else if self
            .panels
            .iter()
            .any(|panel| matches!(panel.state, AgentState::Busy))
            || self
                .app_thread_activity
                .values()
                .any(|thread| matches!(thread.state, AgentState::Busy))
        {
            Duration::from_millis(33)
        } else {
            Duration::from_millis(120)
        }
    }

    pub fn busy_animation_interval(&self) -> Duration {
        Duration::from_millis(33)
    }

    pub fn begin_preview_mouse_selection(&mut self, column: u16, row: u16) {
        self.preview_mouse_selection = Some(PreviewMouseSelection {
            anchor_column: column,
            anchor_row: row,
            current_column: column,
            current_row: row,
        });
        self.dirty = true;
    }

    pub fn update_preview_mouse_selection(&mut self, column: u16, row: u16) -> bool {
        let Some(selection) = self.preview_mouse_selection.as_mut() else {
            return false;
        };

        if selection.current_column == column && selection.current_row == row {
            return false;
        }

        selection.current_column = column;
        selection.current_row = row;
        self.dirty = true;
        true
    }

    pub fn preview_mouse_selection(&self) -> Option<&PreviewMouseSelection> {
        self.preview_mouse_selection.as_ref()
    }

    pub fn clear_preview_mouse_selection(&mut self) -> bool {
        if self.preview_mouse_selection.take().is_some() {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn finish_preview_mouse_selection(&mut self) -> Option<PreviewMouseSelection> {
        let selection = self.preview_mouse_selection.take();
        if selection.is_some() {
            self.dirty = true;
        }
        selection
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::FocusTarget;
    use crate::app::{
        App, PreviewDetailCache, ThreadPreviewCacheEntry, THREAD_PREVIEW_CACHE_MAX_ENTRIES,
    };
    use crate::model::{
        AgentPanel, AgentState, AgentStateSource, AgentType, PreviewSource, PreviewTurn,
        PreviewView, SessionCacheState,
    };
    use crate::preview_source::PreviewUpdate;
    use crate::sidebar::ThreadActivityOverride;
    use ratatui::text::Line;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc;

    #[test]
    fn open_latest_preview_turn_uses_selected_panel_cached_turns() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "main".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            is_active: true,
            state: AgentState::Busy,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("- item".into()),
            }],
            session_cache_state: Some(SessionCacheState::Cached),
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });

        app.preview_source = PreviewSource::Session;
        app.preview_pane_id = Some("%other".into());
        app.preview_turns = vec![PreviewTurn {
            question: "stale".into(),
            answer: Some("stale".into()),
        }];

        assert!(app.open_latest_preview_turn());
        assert_eq!(app.preview_pane_id.as_deref(), Some("live:%1"));
        assert_eq!(app.preview_selected_turn, Some(0));
        assert_eq!(app.preview_expanded_turn, Some(0));
        assert_eq!(app.preview_turns[0].question, "latest");
    }

    #[test]
    fn historical_detail_pauses_background_preview_refresh() {
        let mut app = App::new();
        app.preview_source = PreviewSource::Session;
        app.preview_turns = vec![
            PreviewTurn {
                question: "latest".into(),
                answer: Some("latest answer".into()),
            },
            PreviewTurn {
                question: "older".into(),
                answer: Some("older answer".into()),
            },
        ];
        app.preview_selected_turn = Some(1);
        app.preview_expanded_turn = Some(1);
        app.preview_view = PreviewView::SessionDetail;

        assert!(app.should_pause_preview_refresh());

        app.preview_selected_turn = Some(0);
        app.preview_expanded_turn = Some(0);
        assert!(app.should_pause_preview_refresh());
    }

    #[test]
    fn identical_preview_update_preserves_detail_cache() {
        let mut app = App::new();
        let turns = vec![PreviewTurn {
            question: "latest".into(),
            answer: Some("latest answer".into()),
        }];
        app.preview_source = PreviewSource::Session;
        app.preview_pane_id = Some("live:%1".into());
        app.preview_turns = turns.clone();
        app.preview_selected_turn = Some(0);
        app.preview_expanded_turn = Some(0);
        app.preview_detail_cache = Some(PreviewDetailCache {
            target_key: "live:%1".into(),
            turn_index: 0,
            width: 80,
            theme_name: "matrix".into(),
            question: "latest".into(),
            answer: Some("latest answer".into()),
            lines: vec![Line::from("cached")],
        });

        let (tx, rx) = mpsc::channel(1);
        tx.blocking_send(PreviewUpdate {
            target_key: "live:%1".into(),
            live_pane_id: Some("%1".into()),
            content: "latest\nlatest answer".into(),
            source: PreviewSource::Session,
            session_origin: None,
            session_id: Some("session-1".into()),
            turns,
            transcript_path: None,
            session_cache_state: Some(SessionCacheState::Cached),
            updated_at: None,
        })
        .unwrap();
        app.preview_rx = Some(rx);

        app.check_preview_result();

        assert!(app.preview_detail_cache.is_some());
        assert_eq!(
            app.preview_detail_cache
                .as_ref()
                .and_then(|cache| cache.lines.first())
                .map(|line| line.to_string()),
            Some("cached".to_string())
        );
    }

    #[test]
    fn expanded_preview_focus_pauses_busy_animations() {
        let mut app = App::new();
        app.preview_focus = FocusTarget::Preview;
        app.preview_expanded_turn = Some(0);
        app.preview_view = PreviewView::SessionDetail;
        assert!(app.should_pause_busy_animations());

        app.preview_expanded_turn = None;
        app.preview_view = PreviewView::SessionList;
        assert!(!app.should_pause_busy_animations());

        app.preview_expanded_turn = Some(0);
        app.preview_view = PreviewView::SessionDetail;
        app.preview_focus = FocusTarget::Panel;
        assert!(app.should_pause_busy_animations());
    }

    #[test]
    fn slow_frame_only_slows_busy_animation_instead_of_stopping_it() {
        let mut app = App::new();
        app.preview_view = PreviewView::SessionList;
        app.frame_budget_exceeded = true;
        app.last_busy_animation_tick = Instant::now() - Duration::from_secs(1);
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "1".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            is_active: true,
            state: AgentState::Busy,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });

        assert!(app.should_tick_busy_animation());
    }

    #[test]
    fn app_only_busy_thread_keeps_busy_animation_ticking() {
        let mut app = App::new();
        app.preview_view = PreviewView::SessionList;
        app.last_busy_animation_tick = Instant::now() - Duration::from_secs(1);
        app.app_thread_activity.insert(
            "codex:app-thread".into(),
            ThreadActivityOverride {
                agent_type: AgentType::Codex,
                session_id: Some("app-thread".into()),
                transcript_path: None,
                working_dir: "/tmp/demo".into(),
                state: AgentState::Busy,
                is_active: true,
                last_user_prompt: None,
                last_assistant_message: None,
                updated_at: 1,
            },
        );

        assert!(app.should_tick_busy_animation());
    }

    #[test]
    fn thread_preview_cache_prunes_to_max_entries() {
        let mut app = App::new();
        let base_ts = 1_000_000i64;
        let total = THREAD_PREVIEW_CACHE_MAX_ENTRIES + 8;
        for i in 0..total {
            let ts = base_ts + i as i64;
            app.thread_preview_cache.insert(
                format!("thread:{}", i),
                ThreadPreviewCacheEntry {
                    turns: Vec::new(),
                    session_cache_state: None,
                    transcript_path: None,
                    session_id: None,
                    updated_at: Some(ts),
                    cached_at: ts,
                },
            );
        }

        assert!(app.prune_thread_preview_cache());
        assert_eq!(
            app.thread_preview_cache.len(),
            THREAD_PREVIEW_CACHE_MAX_ENTRIES
        );
        assert!(app
            .thread_preview_cache
            .contains_key(&format!("thread:{}", total - 1)));
        assert!(!app.thread_preview_cache.contains_key("thread:0"));
    }
}
