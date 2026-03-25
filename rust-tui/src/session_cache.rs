use crate::hook::HookEvent;
use crate::model::{
    AgentPanel, AgentState, AgentStateSource, AgentType, PreviewTurn, SessionCacheState,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_VERSION: u32 = 1;
const RETENTION_SECS: i64 = 30 * 24 * 60 * 60;
pub const SESSION_HISTORY_TURN_LIMIT: usize = 50;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionCacheSnapshot {
    pub agent_session_id: String,
    pub transcript_path: Option<String>,
    pub recent_turns: Vec<PreviewTurn>,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub state: SessionCacheState,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct SessionCacheIndex {
    version: u32,
    sessions: Vec<CachedSessionRecord>,
    pane_bindings: Vec<CachedPaneBinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedSessionRecord {
    agent_session_id: String,
    agent_type: String,
    transcript_path: Option<String>,
    recent_turns: Vec<PreviewTurn>,
    last_user_prompt: Option<String>,
    last_assistant_message: Option<String>,
    last_seen_at: i64,
    updated_at: i64,
    last_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedPaneBinding {
    agent_session_id: String,
    pane_id: String,
    session_name: String,
    window_index: String,
    pane_index: String,
    path: String,
    agent_type: String,
    updated_at: i64,
}

pub fn preload_panels(panels: &mut [AgentPanel]) -> io::Result<()> {
    let mut index = load_index();
    let changed = prune_index(&mut index);

    for panel in panels.iter_mut() {
        if !supports_cached_session(panel) {
            continue;
        }
        if panel.agent_session_id.is_some()
            || panel.transcript_path.is_some()
            || !panel.cached_preview_turns.is_empty()
        {
            continue;
        }

        if let Some(snapshot) = find_snapshot_for_panel(&index, panel) {
            apply_snapshot_to_panel(panel, &snapshot);
        }
    }

    if changed {
        save_index(&index)?;
    }

    Ok(())
}

pub fn persist_hook_event(
    panel: &AgentPanel,
    event: &HookEvent,
) -> io::Result<Option<SessionCacheSnapshot>> {
    let Some(agent_session_id) = event
        .session_id
        .clone()
        .or_else(|| panel.agent_session_id.clone())
    else {
        return Ok(None);
    };

    let mut index = load_index();
    let _ = prune_index(&mut index);
    let now = now_ts();
    let agent_type = panel.agent_type.to_string();

    let record_idx = upsert_session_record(&mut index, &agent_session_id, &agent_type, now);
    index.sessions[record_idx].transcript_path = prefer_non_empty(
        event.transcript_path.as_ref(),
        panel.transcript_path.as_ref(),
        index.sessions[record_idx].transcript_path.as_ref(),
    );
    index.sessions[record_idx].last_source = "hook".to_string();
    index.sessions[record_idx].last_seen_at = now;
    index.sessions[record_idx].updated_at = now;

    let prompt = clean_text(event.prompt.as_deref())
        .or_else(|| clean_text(panel.last_user_prompt.as_deref()));
    let assistant = clean_text(event.last_assistant_message.as_deref())
        .or_else(|| clean_text(panel.last_assistant_message.as_deref()));

    merge_recent_turns(
        &mut index.sessions[record_idx].recent_turns,
        prompt.as_deref(),
        assistant.as_deref(),
        clean_text(panel.last_user_prompt.as_deref()).as_deref(),
    );

    if let Some(first) = index.sessions[record_idx].recent_turns.first().cloned() {
        index.sessions[record_idx].last_user_prompt = Some(first.question);
        index.sessions[record_idx].last_assistant_message = first.answer;
    } else {
        index.sessions[record_idx].last_user_prompt = prompt.clone();
        index.sessions[record_idx].last_assistant_message = assistant.clone();
    }

    upsert_binding(
        &mut index,
        panel,
        &agent_session_id,
        HookBindingContext::from_event(event),
        now,
    );
    save_index(&index)?;

    Ok(Some(snapshot_from_record(
        &index.sessions[record_idx],
        SessionCacheState::Confirmed,
    )))
}

pub fn persist_resolved_session(
    panel: &AgentPanel,
    transcript_path: &Path,
    turns: &[PreviewTurn],
) -> io::Result<()> {
    let Some(agent_session_id) = panel.agent_session_id.as_ref() else {
        return Ok(());
    };

    let normalized_turns = normalize_turns(turns.to_vec());
    let transcript = transcript_path.to_string_lossy().to_string();
    if panel.session_cache_state == Some(SessionCacheState::Confirmed)
        && panel.transcript_path.as_deref() == Some(transcript.as_str())
        && panel.cached_preview_turns == normalized_turns
    {
        return Ok(());
    }

    let mut index = load_index();
    let _ = prune_index(&mut index);
    let now = now_ts();
    let agent_type = panel.agent_type.to_string();
    let record_idx = upsert_session_record(&mut index, agent_session_id, &agent_type, now);
    index.sessions[record_idx].transcript_path = Some(transcript);
    index.sessions[record_idx].recent_turns = normalized_turns.clone();
    index.sessions[record_idx].last_user_prompt =
        normalized_turns.first().map(|turn| turn.question.clone());
    index.sessions[record_idx].last_assistant_message = normalized_turns
        .first()
        .and_then(|turn| turn.answer.clone());
    index.sessions[record_idx].last_source = "resolver".to_string();
    index.sessions[record_idx].last_seen_at = now;
    index.sessions[record_idx].updated_at = now;

    upsert_binding(
        &mut index,
        panel,
        agent_session_id,
        HookBindingContext::default(),
        now,
    );
    save_index(&index)
}

fn supports_cached_session(panel: &AgentPanel) -> bool {
    matches!(panel.agent_type, AgentType::Claude | AgentType::Codex)
}

fn find_snapshot_for_panel(
    index: &SessionCacheIndex,
    panel: &AgentPanel,
) -> Option<SessionCacheSnapshot> {
    let agent_type = panel.agent_type.to_string();

    let exact_matches = index
        .pane_bindings
        .iter()
        .filter(|binding| binding.pane_id == panel.pane_id && binding.agent_type == agent_type)
        .map(|binding| binding.agent_session_id.clone())
        .collect::<HashSet<_>>();

    if exact_matches.len() == 1 {
        let session_id = exact_matches.iter().next()?;
        return lookup_snapshot(index, session_id, SessionCacheState::Cached);
    }

    let fallback_matches = index
        .pane_bindings
        .iter()
        .filter(|binding| {
            binding.agent_type == agent_type
                && binding.path == panel.working_dir
                && binding.session_name == panel.session
                && binding.window_index == panel.window_index
                && binding.pane_index == panel.pane
        })
        .map(|binding| binding.agent_session_id.clone())
        .collect::<HashSet<_>>();

    if fallback_matches.len() == 1 {
        let session_id = fallback_matches.iter().next()?;
        return lookup_snapshot(index, session_id, SessionCacheState::Cached);
    }

    None
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

fn apply_snapshot_to_panel(panel: &mut AgentPanel, snapshot: &SessionCacheSnapshot) {
    panel.agent_session_id = Some(snapshot.agent_session_id.clone());
    panel.transcript_path = snapshot.transcript_path.clone();
    panel.cached_preview_turns = snapshot.recent_turns.clone();
    panel.last_user_prompt = snapshot.last_user_prompt.clone();
    panel.last_assistant_message = snapshot.last_assistant_message.clone();
    panel.session_cache_state = Some(snapshot.state);

    if latest_turn_missing_answer(&snapshot.recent_turns) {
        panel.state = AgentState::Busy;
        panel.state_source = AgentStateSource::Hook;
        panel.is_active = true;
    }
}

fn latest_turn_missing_answer(turns: &[PreviewTurn]) -> bool {
    turns
        .first()
        .map(|turn| {
            !turn.question.trim().is_empty()
                && turn
                    .answer
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
        })
        .unwrap_or(false)
}

fn snapshot_from_record(
    record: &CachedSessionRecord,
    state: SessionCacheState,
) -> SessionCacheSnapshot {
    SessionCacheSnapshot {
        agent_session_id: record.agent_session_id.clone(),
        transcript_path: record.transcript_path.clone(),
        recent_turns: record.recent_turns.clone(),
        last_user_prompt: record.last_user_prompt.clone(),
        last_assistant_message: record.last_assistant_message.clone(),
        state,
    }
}

fn upsert_session_record(
    index: &mut SessionCacheIndex,
    agent_session_id: &str,
    agent_type: &str,
    now: i64,
) -> usize {
    if let Some(existing_idx) = index
        .sessions
        .iter()
        .position(|record| record.agent_session_id == agent_session_id)
    {
        index.sessions[existing_idx].agent_type = agent_type.to_string();
        index.sessions[existing_idx].last_seen_at = now;
        return existing_idx;
    }

    index.sessions.push(CachedSessionRecord {
        agent_session_id: agent_session_id.to_string(),
        agent_type: agent_type.to_string(),
        transcript_path: None,
        recent_turns: Vec::new(),
        last_user_prompt: None,
        last_assistant_message: None,
        last_seen_at: now,
        updated_at: now,
        last_source: "hook".to_string(),
    });
    index.sessions.len().saturating_sub(1)
}

#[derive(Default)]
struct HookBindingContext {
    session_name: Option<String>,
    window_index: Option<String>,
    pane_index: Option<String>,
    path: Option<String>,
}

impl HookBindingContext {
    fn from_event(event: &HookEvent) -> Self {
        Self {
            session_name: event.tmux.session_name.clone(),
            window_index: event.tmux.window_index.clone(),
            pane_index: event.tmux.pane_index.clone(),
            path: event
                .tmux
                .pane_current_path
                .clone()
                .or_else(|| event.cwd.clone()),
        }
    }
}

fn upsert_binding(
    index: &mut SessionCacheIndex,
    panel: &AgentPanel,
    agent_session_id: &str,
    ctx: HookBindingContext,
    now: i64,
) {
    let binding = CachedPaneBinding {
        agent_session_id: agent_session_id.to_string(),
        pane_id: panel.pane_id.clone(),
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

fn merge_recent_turns(
    turns: &mut Vec<PreviewTurn>,
    prompt: Option<&str>,
    assistant: Option<&str>,
    fallback_question: Option<&str>,
) {
    let prompt = clean_text(prompt);
    let assistant = clean_text(assistant);
    let fallback_question = clean_text(fallback_question);

    if let Some(prompt_text) = prompt.as_deref() {
        let should_insert = match turns.first() {
            Some(first) => first.question.trim() != prompt_text || first.answer.is_some(),
            None => true,
        };
        if should_insert {
            turns.insert(
                0,
                PreviewTurn {
                    question: prompt_text.to_string(),
                    answer: None,
                },
            );
        }
    }

    if let Some(answer_text) = assistant.as_deref() {
        let question_hint = prompt.as_deref().or(fallback_question.as_deref());
        if let Some(first) = turns.first_mut() {
            let question_matches = question_hint
                .map(|hint| first.question.trim() == hint)
                .unwrap_or(true);
            if question_matches || first.answer.is_none() {
                first.answer = Some(answer_text.to_string());
            } else if let Some(hint) = question_hint {
                turns.insert(
                    0,
                    PreviewTurn {
                        question: hint.to_string(),
                        answer: Some(answer_text.to_string()),
                    },
                );
            }
        } else if let Some(hint) = question_hint {
            turns.push(PreviewTurn {
                question: hint.to_string(),
                answer: Some(answer_text.to_string()),
            });
        }
    }

    *turns = normalize_turns(std::mem::take(turns));
}

fn normalize_turns(turns: Vec<PreviewTurn>) -> Vec<PreviewTurn> {
    let mut normalized = turns
        .into_iter()
        .filter_map(|turn| {
            let question = turn.question.trim().to_string();
            if question.is_empty() {
                return None;
            }
            let answer = turn
                .answer
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(ToOwned::to_owned);
            Some(PreviewTurn { question, answer })
        })
        .collect::<Vec<_>>();

    if normalized.len() > SESSION_HISTORY_TURN_LIMIT {
        normalized.truncate(SESSION_HISTORY_TURN_LIMIT);
    }

    normalized
}

fn prune_index(index: &mut SessionCacheIndex) -> bool {
    if index.version != CACHE_VERSION {
        index.version = CACHE_VERSION;
    }

    let now = now_ts();
    let min_ts = now.saturating_sub(RETENTION_SECS);

    let before_sessions = index.sessions.len();
    index.sessions.retain(|record| {
        if record.updated_at < min_ts {
            return false;
        }
        if let Some(path) = record.transcript_path.as_deref() {
            return Path::new(path).exists();
        }
        true
    });

    let valid_session_ids = index
        .sessions
        .iter()
        .map(|record| record.agent_session_id.clone())
        .collect::<HashSet<_>>();

    let before_bindings = index.pane_bindings.len();
    index.pane_bindings.retain(|binding| {
        binding.updated_at >= min_ts && valid_session_ids.contains(&binding.agent_session_id)
    });

    before_sessions != index.sessions.len() || before_bindings != index.pane_bindings.len()
}

fn load_index() -> SessionCacheIndex {
    let path = crate::paths::sessions_index_path();
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => {
            return SessionCacheIndex {
                version: CACHE_VERSION,
                ..SessionCacheIndex::default()
            }
        }
    };

    serde_json::from_str(&content).unwrap_or_else(|err| {
        log_debug!("session_cache: failed to parse {}: {}", path.display(), err);
        SessionCacheIndex {
            version: CACHE_VERSION,
            ..SessionCacheIndex::default()
        }
    })
}

fn save_index(index: &SessionCacheIndex) -> io::Result<()> {
    let path = crate::paths::sessions_index_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = temporary_index_path(&path);
    let content = serde_json::to_string_pretty(index)?;
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

fn temporary_index_path(path: &Path) -> PathBuf {
    let pid = std::process::id();
    let stamp = now_ts();
    path.with_extension(format!("tmp.{}.{}", pid, stamp))
}

fn prefer_non_empty(
    first: Option<&String>,
    second: Option<&String>,
    third: Option<&String>,
) -> Option<String> {
    first
        .and_then(|value| clean_text(Some(value.as_str())))
        .or_else(|| second.and_then(|value| clean_text(Some(value.as_str()))))
        .or_else(|| third.and_then(|value| clean_text(Some(value.as_str()))))
}

fn clean_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_snapshot_to_panel, find_snapshot_for_panel, latest_turn_missing_answer,
        merge_recent_turns, snapshot_from_record, CachedPaneBinding, CachedSessionRecord,
        SessionCacheIndex, SessionCacheSnapshot,
    };
    use crate::model::{
        AgentPanel, AgentState, AgentStateSource, AgentType, PreviewTurn, SessionCacheState,
    };

    fn panel(
        pane_id: &str,
        session: &str,
        window_index: &str,
        pane: &str,
        path: &str,
    ) -> AgentPanel {
        AgentPanel {
            session: session.to_string(),
            window: "win".to_string(),
            window_index: window_index.to_string(),
            pane: pane.to_string(),
            pane_id: pane_id.to_string(),
            agent_type: AgentType::Codex,
            working_dir: path.to_string(),
            is_active: false,
            state: AgentState::Idle,
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
        }
    }

    #[test]
    fn merge_recent_turns_prefers_latest_prompt_and_answer() {
        let mut turns = Vec::new();
        merge_recent_turns(&mut turns, Some("hello"), None, None);
        merge_recent_turns(&mut turns, None, Some("world"), Some("hello"));
        assert_eq!(
            turns,
            vec![PreviewTurn {
                question: "hello".to_string(),
                answer: Some("world".to_string()),
            }]
        );
    }

    #[test]
    fn fallback_match_is_ambiguous_when_multiple_sessions_share_same_slot() {
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![
                CachedSessionRecord {
                    agent_session_id: "s1".to_string(),
                    agent_type: "codex".to_string(),
                    transcript_path: Some("/tmp/a.jsonl".to_string()),
                    recent_turns: vec![PreviewTurn {
                        question: "q1".to_string(),
                        answer: None,
                    }],
                    last_user_prompt: None,
                    last_assistant_message: None,
                    last_seen_at: 1,
                    updated_at: 1,
                    last_source: "hook".to_string(),
                },
                CachedSessionRecord {
                    agent_session_id: "s2".to_string(),
                    agent_type: "codex".to_string(),
                    transcript_path: Some("/tmp/b.jsonl".to_string()),
                    recent_turns: vec![PreviewTurn {
                        question: "q2".to_string(),
                        answer: None,
                    }],
                    last_user_prompt: None,
                    last_assistant_message: None,
                    last_seen_at: 1,
                    updated_at: 1,
                    last_source: "hook".to_string(),
                },
            ],
            pane_bindings: vec![
                CachedPaneBinding {
                    agent_session_id: "s1".to_string(),
                    pane_id: "%1".to_string(),
                    session_name: "dev".to_string(),
                    window_index: "1".to_string(),
                    pane_index: "0".to_string(),
                    path: "/repo".to_string(),
                    agent_type: "codex".to_string(),
                    updated_at: 1,
                },
                CachedPaneBinding {
                    agent_session_id: "s2".to_string(),
                    pane_id: "%2".to_string(),
                    session_name: "dev".to_string(),
                    window_index: "1".to_string(),
                    pane_index: "0".to_string(),
                    path: "/repo".to_string(),
                    agent_type: "codex".to_string(),
                    updated_at: 1,
                },
            ],
        };

        assert!(find_snapshot_for_panel(&index, &panel("%9", "dev", "1", "0", "/repo")).is_none());
    }

    #[test]
    fn exact_pane_match_wins_even_if_slot_history_is_ambiguous() {
        let record = CachedSessionRecord {
            agent_session_id: "s1".to_string(),
            agent_type: "codex".to_string(),
            transcript_path: Some("/tmp/a.jsonl".to_string()),
            recent_turns: vec![PreviewTurn {
                question: "q1".to_string(),
                answer: None,
            }],
            last_user_prompt: None,
            last_assistant_message: None,
            last_seen_at: 1,
            updated_at: 1,
            last_source: "hook".to_string(),
        };

        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![record.clone()],
            pane_bindings: vec![CachedPaneBinding {
                agent_session_id: "s1".to_string(),
                pane_id: "%1".to_string(),
                session_name: "dev".to_string(),
                window_index: "1".to_string(),
                pane_index: "0".to_string(),
                path: "/repo".to_string(),
                agent_type: "codex".to_string(),
                updated_at: 1,
            }],
        };

        let snapshot =
            find_snapshot_for_panel(&index, &panel("%1", "other", "9", "9", "/else")).unwrap();
        assert_eq!(
            snapshot,
            snapshot_from_record(&record, SessionCacheState::Cached)
        );
    }

    #[test]
    fn latest_unanswered_turn_restores_busy_state() {
        let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
        let snapshot = SessionCacheSnapshot {
            agent_session_id: "s1".to_string(),
            transcript_path: Some("/tmp/a.jsonl".to_string()),
            recent_turns: vec![PreviewTurn {
                question: "still running".to_string(),
                answer: None,
            }],
            last_user_prompt: Some("still running".to_string()),
            last_assistant_message: None,
            state: SessionCacheState::Cached,
        };

        apply_snapshot_to_panel(&mut restored_panel, &snapshot);

        assert_eq!(restored_panel.state, AgentState::Busy);
        assert_eq!(restored_panel.state_source, AgentStateSource::Hook);
        assert!(restored_panel.is_active);
    }

    #[test]
    fn answered_latest_turn_does_not_force_busy_state() {
        let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
        let snapshot = SessionCacheSnapshot {
            agent_session_id: "s1".to_string(),
            transcript_path: Some("/tmp/a.jsonl".to_string()),
            recent_turns: vec![PreviewTurn {
                question: "done".to_string(),
                answer: Some("finished".to_string()),
            }],
            last_user_prompt: Some("done".to_string()),
            last_assistant_message: Some("finished".to_string()),
            state: SessionCacheState::Cached,
        };

        apply_snapshot_to_panel(&mut restored_panel, &snapshot);

        assert_eq!(restored_panel.state, AgentState::Idle);
        assert_eq!(restored_panel.state_source, AgentStateSource::Scanner);
        assert!(!restored_panel.is_active);
    }

    #[test]
    fn latest_turn_missing_answer_only_when_newest_turn_is_unresolved() {
        assert!(latest_turn_missing_answer(&[PreviewTurn {
            question: "pending".to_string(),
            answer: None,
        }]));
        assert!(!latest_turn_missing_answer(&[PreviewTurn {
            question: "done".to_string(),
            answer: Some("answer".to_string()),
        }]));
        assert!(!latest_turn_missing_answer(&[
            PreviewTurn {
                question: "done".to_string(),
                answer: Some("answer".to_string()),
            },
            PreviewTurn {
                question: "old pending".to_string(),
                answer: None,
            },
        ]));
    }
}
