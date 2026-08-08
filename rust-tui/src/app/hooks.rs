mod activity;
mod app_thread;
mod claude_history {
    use super::unix_now_ts;
    use crate::hook::HookEvent;
    use crate::model::{AgentPanel, AgentType};
    use std::path::PathBuf;

    pub(super) fn pane_claude_history_upsert_args(
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
}
mod notification;
mod notification_text;
mod pane;
mod title_summary {
    use crate::hook::HookEvent;
    use crate::model::{AgentPanel, AgentType};

    #[derive(Clone, Debug)]
    pub(super) struct PendingCodexTitleSummary {
        pub(super) session_id: String,
        pub(super) turns: Vec<crate::model::PreviewTurn>,
        pub(super) turn_count: usize,
    }

    pub(super) fn codex_title_summary_request_for_panel(
        codex_config: &crate::theme::CodexConfig,
        panel: &AgentPanel,
        event: &HookEvent,
        persisted_snapshot: Option<&crate::session_cache::SessionCacheSnapshot>,
    ) -> Option<PendingCodexTitleSummary> {
        if !crate::title_summary::is_enabled(codex_config)
            || event.event != "stop"
            || panel.agent_type != AgentType::Codex
        {
            return None;
        }

        let session_id = panel
            .agent_session_id
            .clone()
            .or_else(|| persisted_snapshot.map(|snapshot| snapshot.agent_session_id.clone()))
            .or_else(|| event.session_id.clone())?;

        let turns = persisted_snapshot
            .map(|snapshot| snapshot.recent_turns.to_vec())
            .unwrap_or_else(|| panel.cached_preview_turns.to_vec());
        let turn_count = turns
            .iter()
            .filter(|turn| !turn.question.trim().is_empty())
            .count();

        Some(PendingCodexTitleSummary {
            session_id,
            turns,
            turn_count,
        })
    }
}

use super::{unix_now_ts, App, APP_THREAD_ACTIVITY_MAX_ENTRIES, APP_THREAD_ACTIVITY_TTL_SECS};
use crate::hook::HookEvent;

impl App {
    pub fn apply_hook_event(&mut self, event: HookEvent) {
        activity::normalize_codex_rollout_paths_if_needed(&event);

        let Some(pane_id) = event.tmux.pane_id.clone() else {
            self.apply_app_thread_hook_event(event);
            return;
        };

        self.apply_pane_hook_event(event, pane_id);
    }
}

#[cfg(test)]
mod hooks_tests;
