use super::model::{
    snapshot_from_record, CachedPaneBinding, CachedSessionRecord, HookBindingContext,
    SessionCacheIndex, SessionCacheSnapshot,
};
use super::util::now_ts;
use crate::model::{AgentPanel, SessionCacheState};
use std::collections::HashMap;

const RECENT_BINDING_WINDOW_SECS: i64 = 2 * 60 * 60;

pub(super) fn find_snapshot_for_panel(
    index: &SessionCacheIndex,
    panel: &AgentPanel,
) -> Option<SessionCacheSnapshot> {
    let agent_type = panel.agent_type.to_string();
    let now = now_ts();

    let exact_match = find_unique_session_id(
        panel,
        index.pane_bindings.iter().filter_map(|binding| {
            (binding.agent_type == agent_type && exact_binding_matches(binding, panel, now))
                .then_some(binding.agent_session_id.as_str())
        }),
    );

    if let Some(session_id) = exact_match {
        return lookup_snapshot(index, session_id, SessionCacheState::Cached);
    }

    let fallback_match = find_unique_session_id(
        panel,
        index.pane_bindings.iter().filter_map(|binding| {
            (binding.agent_type == agent_type && fallback_binding_matches(binding, panel, now))
                .then_some(binding.agent_session_id.as_str())
        }),
    );

    if let Some(session_id) = fallback_match {
        return lookup_snapshot(index, session_id, SessionCacheState::Cached);
    }

    None
}

pub(super) fn load_snapshots_for_agent_type(
    index: &SessionCacheIndex,
    agent_type: &str,
) -> HashMap<String, SessionCacheSnapshot> {
    index
        .sessions
        .iter()
        .filter(|record| record.agent_type == agent_type)
        .map(|record| {
            (
                record.agent_session_id.clone(),
                snapshot_from_record(record, snapshot_state(record)),
            )
        })
        .collect()
}

pub(super) fn upsert_binding(
    index: &mut SessionCacheIndex,
    panel: &AgentPanel,
    agent_session_id: &str,
    ctx: HookBindingContext,
    now: i64,
) {
    let binding = CachedPaneBinding {
        agent_session_id: agent_session_id.to_string(),
        pane_id: panel.pane_id.clone(),
        pane_pid: panel.pid.clone(),
        session_name: ctx.session_name.unwrap_or_else(|| panel.session.clone()),
        window_index: ctx
            .window_index
            .unwrap_or_else(|| panel.window_index.clone()),
        pane_index: ctx.pane_index.unwrap_or_else(|| panel.pane.clone()),
        path: ctx.path.unwrap_or_else(|| panel.working_dir.clone()),
        agent_type: panel.agent_type.to_string(),
        updated_at: now,
    };

    if let Some(existing) = index
        .pane_bindings
        .iter_mut()
        .find(|item| item.pane_id == binding.pane_id)
    {
        *existing = binding;
        return;
    }

    if let Some(existing) = index.pane_bindings.iter_mut().find(|item| {
        item.agent_session_id == binding.agent_session_id
            && item.agent_type == binding.agent_type
            && item.session_name == binding.session_name
            && item.window_index == binding.window_index
            && item.pane_index == binding.pane_index
            && item.path == binding.path
    }) {
        *existing = binding;
        return;
    }

    index.pane_bindings.push(binding);
}

fn exact_binding_matches(binding: &CachedPaneBinding, panel: &AgentPanel, now: i64) -> bool {
    if binding.pane_id != panel.pane_id {
        return false;
    }

    if pane_pid_matches(binding, panel) {
        return true;
    }

    binding_is_recent(binding, now) && binding_matches_slot(binding, panel)
}

fn fallback_binding_matches(binding: &CachedPaneBinding, panel: &AgentPanel, now: i64) -> bool {
    binding_is_recent(binding, now) && binding_matches_slot(binding, panel)
}

fn binding_matches_slot(binding: &CachedPaneBinding, panel: &AgentPanel) -> bool {
    binding.path == panel.working_dir
        && binding.session_name == panel.session
        && binding.window_index == panel.window_index
        && binding.pane_index == panel.pane
}

fn pane_pid_matches(binding: &CachedPaneBinding, panel: &AgentPanel) -> bool {
    match (binding.pane_pid.as_deref(), panel.pid.as_deref()) {
        (Some(binding_pid), Some(panel_pid)) => !binding_pid.is_empty() && binding_pid == panel_pid,
        _ => false,
    }
}

fn binding_is_recent(binding: &CachedPaneBinding, now: i64) -> bool {
    binding.updated_at >= now.saturating_sub(RECENT_BINDING_WINDOW_SECS)
}

fn lookup_snapshot(
    index: &SessionCacheIndex,
    session_id: &str,
    state: SessionCacheState,
) -> Option<SessionCacheSnapshot> {
    index
        .sessions
        .iter()
        .find(|record| record.agent_session_id == session_id)
        .map(|record| snapshot_from_record(record, state))
}

fn snapshot_state(record: &CachedSessionRecord) -> SessionCacheState {
    match record.last_source.as_str() {
        "resolver" => SessionCacheState::Confirmed,
        _ => SessionCacheState::Cached,
    }
}

fn find_unique_session_id<'a>(
    panel: &AgentPanel,
    session_ids: impl Iterator<Item = &'a str>,
) -> Option<&'a str> {
    let mut unique = None;

    for session_id in session_ids {
        if is_subagent_session(panel, session_id) {
            continue;
        }
        match unique {
            None => unique = Some(session_id),
            Some(existing) if existing == session_id => {}
            Some(_) => return None,
        }
    }

    unique
}

fn is_subagent_session(panel: &AgentPanel, session_id: &str) -> bool {
    matches!(panel.agent_type, crate::model::AgentType::Codex)
        && crate::codex_state::subagent_parent_thread_id(session_id)
            .ok()
            .flatten()
            .is_some()
}
