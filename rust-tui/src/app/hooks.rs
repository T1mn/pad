use super::{unix_now_ts, App, APP_THREAD_ACTIVITY_MAX_ENTRIES, APP_THREAD_ACTIVITY_TTL_SECS};
use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType, SessionCacheState};
use crate::sidebar::ThreadActivityOverride;
use std::collections::HashSet;
use std::path::PathBuf;

impl App {
    pub fn apply_hook_event(&mut self, event: HookEvent) {
        let pane_id = match event.tmux.pane_id.clone() {
            Some(id) => id,
            None => {
                self.apply_app_thread_hook_event(event);
                return;
            }
        };
        let panel_item_focused = self.panel_item_is_focused(&pane_id);

        let should_refresh_preview = self
            .selected_panel()
            .map(|panel| panel.pane_id == pane_id)
            .unwrap_or(false);

        let mut persisted_snapshot = None;
        let mut pending_claude_history_upsert = None;

        if let Some(panel) = self.panels.iter_mut().find(|p| p.pane_id == pane_id) {
            if panel.agent_type == AgentType::Codex {
                if let Some(subagent_session_id) = event.session_id.as_deref() {
                    if let Ok(Some(parent_thread_id)) =
                        crate::codex_state::subagent_parent_thread_id(subagent_session_id)
                    {
                        if panel.agent_session_id.is_none()
                            || panel.agent_session_id.as_deref() == Some(subagent_session_id)
                        {
                            panel.agent_session_id = Some(parent_thread_id.clone());
                        }
                        log_debug!(
                            "hook: ignoring codex subagent event pane={} subagent_session={} parent_session={}",
                            pane_id,
                            subagent_session_id,
                            parent_thread_id
                        );
                        self.invalidate_sidebar_cache();
                        if should_refresh_preview {
                            self.invalidate_preview();
                        }
                        self.dirty = true;
                        return;
                    }
                }
            }

            if event.session_id.is_some() {
                panel.agent_session_id = event.session_id.clone();
            }
            if event.transcript_path.is_some() {
                panel.transcript_path = event.transcript_path.clone();
            }
            match event.event.as_str() {
                "session_start" => {}
                "user_prompt_submit" => {
                    panel.state = AgentState::Busy;
                    panel.state_source = AgentStateSource::Hook;
                    panel.is_active = true;
                    panel.last_user_prompt = event.prompt.clone();
                    panel.has_unread_stop = false;
                }
                "stop" => {
                    panel.state = AgentState::Waiting;
                    panel.state_source = AgentStateSource::Hook;
                    panel.is_active = false;
                    panel.has_unread_stop = !panel_item_focused;
                    if event.last_assistant_message.is_some() {
                        panel.last_assistant_message = event.last_assistant_message.clone();
                    }
                }
                _ => {}
            }

            match crate::session_cache::persist_hook_event(panel, &event) {
                Ok(snapshot) => persisted_snapshot = snapshot,
                Err(err) => log_debug!("session_cache: persist hook failed: {}", err),
            }

            if let Some(snapshot) = persisted_snapshot.as_ref() {
                panel.agent_session_id = Some(snapshot.agent_session_id.clone());
                panel.transcript_path = snapshot.transcript_path.clone();
                panel.cached_preview_turns = snapshot.recent_turns.clone();
                panel.last_user_prompt = snapshot.last_user_prompt.clone();
                panel.last_assistant_message = snapshot.last_assistant_message.clone();
                panel.session_cache_state = Some(SessionCacheState::Confirmed);
            }

            pending_claude_history_upsert =
                pane_claude_history_upsert_args(panel, &event, persisted_snapshot.as_ref());

            self.invalidate_sidebar_cache();
            if should_refresh_preview {
                self.invalidate_preview();
            }
            self.dirty = true;
        }

        if let Some((session_id, transcript_path, cwd, title, updated_at)) =
            pending_claude_history_upsert
        {
            if let Err(err) = crate::claude_history::upsert_hook_session(
                &session_id,
                &transcript_path,
                &cwd,
                title.as_deref(),
                updated_at,
            ) {
                log_debug!("claude_history: pane hook upsert failed: {}", err);
            }
        }
    }

    fn apply_app_thread_hook_event(&mut self, event: HookEvent) {
        let Some(activity) = app_thread_activity_from_hook(&event) else {
            return;
        };

        let selected_matches = self
            .selected_preview_thread()
            .map(|thread| {
                thread.agent_type == activity.agent_type
                    && ((activity.session_id.is_some() && activity.session_id == thread.session_id)
                        || (activity.transcript_path.is_some()
                            && activity.transcript_path == thread.transcript_path)
                        || thread.working_dir == activity.working_dir)
            })
            .unwrap_or(false);

        let key = activity
            .session_id
            .clone()
            .or(activity.transcript_path.clone())
            .unwrap_or_else(|| format!("{}:{}", activity.agent_type, activity.working_dir));
        self.app_thread_activity.insert(key, activity);
        self.prune_app_thread_activity(unix_now_ts());
        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        if selected_matches {
            self.invalidate_preview();
        }
        self.dirty = true;
    }

    pub(crate) fn prune_app_thread_activity(&mut self, now_ts: i64) -> bool {
        let cutoff = now_ts.saturating_sub(APP_THREAD_ACTIVITY_TTL_SECS);
        let before = self.app_thread_activity.len();
        self.app_thread_activity
            .retain(|_, activity| activity.updated_at >= cutoff);

        if self.app_thread_activity.len() > APP_THREAD_ACTIVITY_MAX_ENTRIES {
            let mut keys_by_freshness = self
                .app_thread_activity
                .iter()
                .map(|(key, activity)| (key.clone(), activity.updated_at))
                .collect::<Vec<_>>();
            keys_by_freshness
                .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            let keep = keys_by_freshness
                .into_iter()
                .take(APP_THREAD_ACTIVITY_MAX_ENTRIES)
                .map(|item| item.0)
                .collect::<HashSet<_>>();
            self.app_thread_activity.retain(|key, _| keep.contains(key));
        }

        self.app_thread_activity.len() != before
    }

    fn panel_item_is_focused(&mut self, pane_id: &str) -> bool {
        !self.show_tree
            && self.preview_focus == super::state::FocusTarget::Panel
            && self
                .selected_preview_thread()
                .and_then(|thread| thread.live_pane_id)
                .map(|selected_pane_id| selected_pane_id == pane_id)
                .unwrap_or(false)
    }

    pub fn clear_unread_stop_for_selected_panel(&mut self) {
        if self.show_tree || self.preview_focus != super::state::FocusTarget::Panel {
            return;
        }

        let Some(selected_pane_id) = self
            .selected_preview_thread()
            .and_then(|thread| thread.live_pane_id)
        else {
            return;
        };

        if let Some(panel) = self
            .panels
            .iter_mut()
            .find(|panel| panel.pane_id == selected_pane_id)
        {
            panel.has_unread_stop = false;
        }
    }
}

fn app_thread_activity_from_hook(event: &HookEvent) -> Option<ThreadActivityOverride> {
    let working_dir = event.cwd.clone()?;
    let agent_type = infer_hook_agent_type(event)?;
    let updated_at = unix_now_ts();

    let (state, is_active) = match event.event.as_str() {
        "user_prompt_submit" => (AgentState::Busy, true),
        "stop" => (AgentState::Waiting, false),
        "session_start" => (AgentState::Idle, false),
        _ => (AgentState::Idle, false),
    };

    Some(ThreadActivityOverride {
        agent_type,
        session_id: event.session_id.clone(),
        transcript_path: event.transcript_path.clone(),
        working_dir,
        state,
        is_active,
        last_user_prompt: event.prompt.clone(),
        last_assistant_message: event.last_assistant_message.clone(),
        updated_at,
    })
}

fn infer_hook_agent_type(event: &HookEvent) -> Option<AgentType> {
    if let Some(path) = event.transcript_path.as_deref() {
        if path.contains("/.codex/") {
            return Some(AgentType::Codex);
        }
        if path.contains("/.claude/") {
            return Some(AgentType::Claude);
        }
    }

    if let Some(session_id) = event.session_id.as_deref() {
        if crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some()
        {
            return Some(AgentType::Codex);
        }
    }

    Some(AgentType::Codex)
}

fn pane_claude_history_upsert_args(
    panel: &AgentPanel,
    event: &HookEvent,
    persisted_snapshot: Option<&crate::session_cache::SessionCacheSnapshot>,
) -> Option<(String, PathBuf, PathBuf, Option<String>, i64)> {
    if panel.agent_type != AgentType::Claude {
        return None;
    }

    let session_id = event
        .session_id
        .clone()
        .or_else(|| persisted_snapshot.map(|snapshot| snapshot.agent_session_id.clone()))
        .or_else(|| panel.agent_session_id.clone())?;

    let transcript_path = event
        .transcript_path
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| {
            persisted_snapshot
                .and_then(|snapshot| snapshot.transcript_path.as_ref())
                .map(PathBuf::from)
        })
        .or_else(|| panel.transcript_path.as_ref().map(PathBuf::from))?;

    let cwd = event
        .cwd
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(panel.working_dir.clone()));

    let title = event
        .prompt
        .clone()
        .or_else(|| persisted_snapshot.and_then(|snapshot| snapshot.last_user_prompt.clone()))
        .or_else(|| panel.last_user_prompt.clone());

    Some((session_id, transcript_path, cwd, title, unix_now_ts()))
}

#[cfg(test)]
mod tests {
    use crate::app::state::FocusTarget;
    use crate::app::{App, APP_THREAD_ACTIVITY_MAX_ENTRIES, APP_THREAD_ACTIVITY_TTL_SECS};
    use crate::hook::{HookEvent, HookTmuxInfo};
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
    use crate::sidebar::ThreadActivityOverride;

    fn stop_event(pane_id: &str) -> HookEvent {
        HookEvent {
            event: "stop".into(),
            session_id: Some("session-1".into()),
            transcript_path: None,
            cwd: None,
            prompt: None,
            last_assistant_message: Some("done".into()),
            timestamp: None,
            tmux: HookTmuxInfo {
                pane_id: Some(pane_id.into()),
                session_name: Some("0".into()),
                window_index: Some("1".into()),
                pane_index: Some("1".into()),
                pane_current_path: Some("/tmp/demo".into()),
            },
        }
    }

    #[test]
    fn stop_hook_marks_panel_unread_when_panel_item_is_not_focused() {
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
        app.table_state.select(Some(0));
        app.preview_focus = FocusTarget::Preview;

        app.apply_hook_event(stop_event("%1"));

        assert!(app.panels[0].has_unread_stop);
    }

    #[test]
    fn focusing_panel_clears_unread_stop_marker() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "main".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            is_active: false,
            state: AgentState::Waiting,
            state_source: AgentStateSource::Hook,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: Some("done".into()),
            has_unread_stop: true,
        });
        app.table_state.select(Some(0));
        app.preview_focus = FocusTarget::Preview;

        app.focus_panel();

        assert!(!app.panels[0].has_unread_stop);
    }

    #[test]
    fn app_thread_activity_prunes_by_ttl_and_cap() {
        let mut app = App::new();
        let now = 2_000_000i64;
        app.app_thread_activity.insert(
            "stale".into(),
            ThreadActivityOverride {
                agent_type: AgentType::Codex,
                session_id: Some("stale".into()),
                transcript_path: None,
                working_dir: "/tmp/stale".into(),
                state: AgentState::Idle,
                is_active: false,
                last_user_prompt: None,
                last_assistant_message: None,
                updated_at: now - APP_THREAD_ACTIVITY_TTL_SECS - 1,
            },
        );
        for i in 0..(APP_THREAD_ACTIVITY_MAX_ENTRIES + 8) {
            app.app_thread_activity.insert(
                format!("recent:{}", i),
                ThreadActivityOverride {
                    agent_type: AgentType::Codex,
                    session_id: Some(format!("recent:{}", i)),
                    transcript_path: None,
                    working_dir: "/tmp/recent".into(),
                    state: AgentState::Busy,
                    is_active: true,
                    last_user_prompt: None,
                    last_assistant_message: None,
                    updated_at: now + i as i64,
                },
            );
        }

        assert!(app.prune_app_thread_activity(now));
        assert!(!app.app_thread_activity.contains_key("stale"));
        assert_eq!(
            app.app_thread_activity.len(),
            APP_THREAD_ACTIVITY_MAX_ENTRIES
        );
        assert!(app
            .app_thread_activity
            .contains_key(&format!("recent:{}", APP_THREAD_ACTIVITY_MAX_ENTRIES + 7)));
        assert!(!app.app_thread_activity.contains_key("recent:0"));
    }
}
