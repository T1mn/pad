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
