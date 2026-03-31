use super::{unix_now_ts, App, APP_THREAD_ACTIVITY_MAX_ENTRIES, APP_THREAD_ACTIVITY_TTL_SECS};
use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType, SessionCacheState};
use crate::notify::NotificationRequest;
use crate::sidebar::{thread_sort_activity_keys, ThreadActivityOverride};
use std::collections::HashSet;
use std::path::Path;
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
        let mut pending_thread_sort_activity = None;
        let mut pending_notification = None;

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
                    panel.last_assistant_message = None;
                    panel.has_unread_stop = false;
                    pending_thread_sort_activity = Some((
                        panel.agent_type.clone(),
                        event
                            .session_id
                            .clone()
                            .or_else(|| panel.agent_session_id.clone()),
                        event
                            .transcript_path
                            .clone()
                            .or_else(|| panel.transcript_path.clone()),
                        panel.working_dir.clone(),
                        unix_now_ts(),
                    ));
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
            pending_notification =
                completion_notification_for_panel(panel, &event, persisted_snapshot.as_ref());

            self.invalidate_sidebar_cache();
            if should_refresh_preview {
                self.invalidate_preview();
            }
            self.dirty = true;
        }

        if let Some((agent_type, session_id, transcript_path, working_dir, now_ts)) =
            pending_thread_sort_activity
        {
            self.record_thread_sort_activity(
                &agent_type,
                session_id.as_deref(),
                transcript_path.as_deref(),
                Some(working_dir.as_str()),
                now_ts,
            );
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

        if let Some(request) = pending_notification {
            emit_completion_notification(request);
        }
    }

    fn apply_app_thread_hook_event(&mut self, event: HookEvent) {
        let Some(activity) = app_thread_activity_from_hook(&event) else {
            return;
        };
        let pending_notification = completion_notification_for_activity(&activity, &event);

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
        if event.event == "user_prompt_submit" {
            self.record_thread_sort_activity(
                &activity.agent_type,
                activity.session_id.as_deref(),
                activity.transcript_path.as_deref(),
                Some(activity.working_dir.as_str()),
                activity.updated_at,
            );
        }
        self.sidebar.app_thread_activity.insert(key, activity);
        self.prune_app_thread_activity(unix_now_ts());
        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        if selected_matches {
            self.invalidate_preview();
        }
        self.dirty = true;

        if let Some(request) = pending_notification {
            emit_completion_notification(request);
        }
    }

    pub(crate) fn prune_app_thread_activity(&mut self, now_ts: i64) -> bool {
        let cutoff = now_ts.saturating_sub(APP_THREAD_ACTIVITY_TTL_SECS);
        let before = self.sidebar.app_thread_activity.len();
        self.sidebar
            .app_thread_activity
            .retain(|_, activity| activity.updated_at >= cutoff);

        if self.sidebar.app_thread_activity.len() > APP_THREAD_ACTIVITY_MAX_ENTRIES {
            let mut keys_by_freshness = self
                .sidebar
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
            self.sidebar
                .app_thread_activity
                .retain(|key, _| keep.contains(key));
        }

        self.sidebar.app_thread_activity.len() != before
    }

    fn record_thread_sort_activity(
        &mut self,
        agent_type: &AgentType,
        session_id: Option<&str>,
        transcript_path: Option<&str>,
        working_dir: Option<&str>,
        now_ts: i64,
    ) {
        for key in thread_sort_activity_keys(agent_type, session_id, transcript_path, working_dir) {
            self.sidebar.thread_sort_activity.insert(key, now_ts);
        }
        self.prune_thread_sort_activity(now_ts);
    }

    fn prune_thread_sort_activity(&mut self, now_ts: i64) {
        let cutoff = now_ts.saturating_sub(APP_THREAD_ACTIVITY_TTL_SECS);
        self.sidebar
            .thread_sort_activity
            .retain(|_, updated_at| *updated_at >= cutoff);

        if self.sidebar.thread_sort_activity.len() <= APP_THREAD_ACTIVITY_MAX_ENTRIES {
            return;
        }

        let mut keys_by_freshness = self
            .sidebar
            .thread_sort_activity
            .iter()
            .map(|(key, updated_at)| (key.clone(), *updated_at))
            .collect::<Vec<_>>();
        keys_by_freshness
            .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        let keep = keys_by_freshness
            .into_iter()
            .take(APP_THREAD_ACTIVITY_MAX_ENTRIES)
            .map(|item| item.0)
            .collect::<HashSet<_>>();
        self.sidebar
            .thread_sort_activity
            .retain(|key, _| keep.contains(key));
    }

    fn panel_item_is_focused(&mut self, pane_id: &str) -> bool {
        !self.sidebar.show_tree
            && self.preview.focus == super::state::FocusTarget::Panel
            && self
                .selected_preview_thread()
                .and_then(|thread| thread.live_pane_id)
                .map(|selected_pane_id| selected_pane_id == pane_id)
                .unwrap_or(false)
    }

    pub fn clear_unread_stop_for_selected_panel(&mut self) {
        if self.sidebar.show_tree || self.preview.focus != super::state::FocusTarget::Panel {
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
        if crate::claude_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some()
        {
            return Some(AgentType::Claude);
        }
        if crate::gemini_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some()
        {
            return Some(AgentType::Gemini);
        }
    }

    Some(AgentType::Codex)
}

fn completion_notification_for_panel(
    panel: &AgentPanel,
    event: &HookEvent,
    persisted_snapshot: Option<&crate::session_cache::SessionCacheSnapshot>,
) -> Option<NotificationRequest> {
    if event.event != "stop" {
        return None;
    }

    let session_id = panel
        .agent_session_id
        .as_deref()
        .or_else(|| persisted_snapshot.map(|snapshot| snapshot.agent_session_id.as_str()))
        .or(event.session_id.as_deref());
    let fallback_prompt = panel
        .last_user_prompt
        .as_deref()
        .or_else(|| persisted_snapshot.and_then(|snapshot| snapshot.last_user_prompt.as_deref()))
        .or(event.prompt.as_deref());

    Some(build_completion_notification(
        &panel.agent_type,
        session_id,
        fallback_prompt,
        Some(panel.working_dir.as_str()),
    ))
}

fn completion_notification_for_activity(
    activity: &ThreadActivityOverride,
    event: &HookEvent,
) -> Option<NotificationRequest> {
    if event.event != "stop" {
        return None;
    }

    Some(build_completion_notification(
        &activity.agent_type,
        activity
            .session_id
            .as_deref()
            .or(event.session_id.as_deref()),
        activity
            .last_user_prompt
            .as_deref()
            .or(event.prompt.as_deref()),
        Some(activity.working_dir.as_str()),
    ))
}

fn build_completion_notification(
    agent_type: &AgentType,
    session_id: Option<&str>,
    fallback_prompt: Option<&str>,
    working_dir: Option<&str>,
) -> NotificationRequest {
    NotificationRequest {
        title: format!("PAD · {} complete", notification_agent_label(agent_type)),
        body: completion_notification_body(agent_type, session_id, fallback_prompt, working_dir),
    }
}

fn emit_completion_notification(request: NotificationRequest) {
    match crate::notify::notify_completion(&request) {
        Ok(true) => {}
        Ok(false) => {
            log_debug!("notification: skipped (no supported desktop backend)");
        }
        Err(err) => {
            log_debug!("notification: failed to dispatch: {}", err);
        }
    }
}

fn notification_agent_label(agent_type: &AgentType) -> &'static str {
    match agent_type {
        AgentType::Claude => "Claude",
        AgentType::Codex => "Codex",
        AgentType::Gemini => "Gemini",
        AgentType::OpenCode => "OpenCode",
        AgentType::Kimi => "Kimi",
        AgentType::Aider => "Aider",
        AgentType::Cursor => "Cursor",
        AgentType::Unknown => "Agent",
    }
}

fn completion_notification_body(
    agent_type: &AgentType,
    session_id: Option<&str>,
    fallback_prompt: Option<&str>,
    working_dir: Option<&str>,
) -> String {
    lookup_notification_title(agent_type, session_id)
        .or_else(|| fallback_prompt.map(normalize_notification_text))
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| notification_workdir_fallback(working_dir, session_id))
}

fn lookup_notification_title(agent_type: &AgentType, session_id: Option<&str>) -> Option<String> {
    let session_id = session_id?;
    match agent_type {
        AgentType::Codex => crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title.or(thread.first_user_message))
            .map(normalize_notification_text),
        AgentType::Claude => crate::claude_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title)
            .map(normalize_notification_text),
        AgentType::Gemini => crate::gemini_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| {
                thread
                    .title
                    .or(thread.summary)
                    .or(thread.last_user_message)
                    .or(thread.first_user_message)
            })
            .map(normalize_notification_text),
        _ => None,
    }
}

fn normalize_notification_text(text: impl AsRef<str>) -> String {
    truncate_notification_text(
        &text
            .as_ref()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        72,
    )
}

fn truncate_notification_text(text: &str, max_chars: usize) -> String {
    let mut truncated = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars {
            truncated.push_str("...");
            return truncated;
        }
        truncated.push(ch);
    }
    truncated
}

fn notification_workdir_fallback(working_dir: Option<&str>, session_id: Option<&str>) -> String {
    working_dir
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .map(normalize_notification_text)
        .filter(|name| !name.is_empty())
        .or_else(|| session_id.map(normalize_notification_text))
        .unwrap_or_else(|| "Session complete".to_string())
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
    use crate::notify::NotificationRequest;
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
        app.preview.focus = FocusTarget::Preview;

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
        app.preview.focus = FocusTarget::Preview;

        app.focus_panel();

        assert!(!app.panels[0].has_unread_stop);
    }

    #[test]
    fn app_thread_activity_prunes_by_ttl_and_cap() {
        let mut app = App::new();
        let now = 2_000_000i64;
        app.sidebar.app_thread_activity.insert(
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
            app.sidebar.app_thread_activity.insert(
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
        assert!(!app.sidebar.app_thread_activity.contains_key("stale"));
        assert_eq!(
            app.sidebar.app_thread_activity.len(),
            APP_THREAD_ACTIVITY_MAX_ENTRIES
        );
        assert!(app
            .sidebar
            .app_thread_activity
            .contains_key(&format!("recent:{}", APP_THREAD_ACTIVITY_MAX_ENTRIES + 7)));
        assert!(!app.sidebar.app_thread_activity.contains_key("recent:0"));
    }

    #[test]
    fn completion_notification_uses_prompt_when_lookup_is_unavailable() {
        let request = super::build_completion_notification(
            &AgentType::Codex,
            Some("missing-session"),
            Some("Ship the relay settings redesign with a compact layout"),
            Some("/tmp/demo"),
        );

        assert_eq!(request.title, "PAD · Codex complete");
        assert_eq!(
            request.body,
            "Ship the relay settings redesign with a compact layout"
        );
    }

    #[test]
    fn completion_notification_falls_back_to_workdir_name() {
        let request = super::build_completion_notification(
            &AgentType::OpenCode,
            None,
            None,
            Some("/tmp/pad-demo"),
        );

        assert_eq!(
            request,
            NotificationRequest {
                title: "PAD · OpenCode complete".into(),
                body: "pad-demo".into(),
            }
        );
    }

    #[test]
    fn completion_notification_truncates_long_text() {
        let body = super::completion_notification_body(
            &AgentType::Unknown,
            None,
            Some(
                "this is a very long prompt that should be truncated before it reaches the desktop notification surface because otherwise it becomes noisy",
            ),
            None,
        );

        assert!(body.ends_with("..."));
        assert!(body.chars().count() <= 75);
    }
}
