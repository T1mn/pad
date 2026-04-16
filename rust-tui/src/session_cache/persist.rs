use super::bindings::upsert_binding;
use super::model::{
    snapshot_from_record, CachedSessionRecord, HookBindingContext, SessionCacheIndex,
    SessionCacheSnapshot, SESSION_HISTORY_TURN_LIMIT,
};
use super::storage::{load_index, prune_index, save_index};
use super::util::{clean_text, now_ts, prefer_non_empty};
use crate::hook::HookEvent;
use crate::model::{AgentPanel, AgentType, PreviewTurn, SessionCacheState};
use crate::session_continuity::ContinuityWriteSource;
use std::borrow::Borrow;
use std::io;
use std::path::Path;

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
    let normalize_codex = panel.agent_type == AgentType::Codex;

    let record_idx = upsert_session_record(&mut index, &agent_session_id, &agent_type, now);
    index.sessions[record_idx].transcript_path = prefer_non_empty(
        event.transcript_path.as_ref(),
        panel.transcript_path.as_ref(),
        index.sessions[record_idx].transcript_path.as_ref(),
    );
    index.sessions[record_idx].last_source = "hook".to_string();
    index.sessions[record_idx].last_seen_at = now;
    index.sessions[record_idx].updated_at = now;

    let prompt = normalize_cached_codex_prompt(
        clean_text(event.prompt.as_deref())
            .or_else(|| clean_text(panel.last_user_prompt.as_deref())),
        normalize_codex,
    );
    let assistant = match event.event.as_str() {
        "user_prompt_submit" => clean_text(event.last_assistant_message.as_deref()),
        _ => clean_text(event.last_assistant_message.as_deref())
            .or_else(|| clean_text(panel.last_assistant_message.as_deref())),
    };

    merge_recent_turns(
        &mut index.sessions[record_idx].recent_turns,
        prompt.as_deref(),
        assistant.as_deref(),
        normalize_cached_codex_prompt(
            clean_text(panel.last_user_prompt.as_deref()),
            normalize_codex,
        )
        .as_deref(),
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

    crate::session_continuity::record_cache_write(
        &panel.agent_type,
        &agent_session_id,
        index.sessions[record_idx]
            .transcript_path
            .as_deref()
            .map(Path::new),
        ContinuityWriteSource::Hook,
        index.sessions[record_idx].recent_turns.len(),
    );

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

    let normalized_turns = normalize_turns(turns, panel.agent_type == AgentType::Codex);
    let transcript = transcript_path.to_string_lossy().to_string();
    if panel.session_cache_state == Some(SessionCacheState::Confirmed)
        && panel.transcript_path.as_deref() == Some(transcript.as_str())
        && panel.cached_preview_turns.as_ref() == normalized_turns.as_slice()
    {
        return Ok(());
    }

    let mut index = load_index();
    let _ = prune_index(&mut index);
    let now = now_ts();
    let agent_type = panel.agent_type.to_string();
    let record_idx = upsert_session_record(&mut index, agent_session_id, &agent_type, now);
    let last_user_prompt = normalized_turns.first().map(|turn| turn.question.clone());
    let last_assistant_message = normalized_turns
        .first()
        .and_then(|turn| turn.answer.clone());
    let record = &mut index.sessions[record_idx];
    record.transcript_path = Some(transcript);
    record.recent_turns = normalized_turns;
    record.last_user_prompt = last_user_prompt;
    record.last_assistant_message = last_assistant_message;
    record.last_source = "resolver".to_string();
    record.last_seen_at = now;
    record.updated_at = now;

    upsert_binding(
        &mut index,
        panel,
        agent_session_id,
        HookBindingContext::default(),
        now,
    );
    save_index(&index)?;
    crate::session_continuity::record_cache_write(
        &panel.agent_type,
        agent_session_id,
        Some(transcript_path),
        ContinuityWriteSource::Resolver,
        index.sessions[record_idx].recent_turns.len(),
    );
    Ok(())
}

pub(super) fn merge_recent_turns(
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

    *turns = normalize_turns(std::mem::take(turns), false);
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

fn normalize_turns<I, T>(turns: I, normalize_codex_prompts: bool) -> Vec<PreviewTurn>
where
    I: IntoIterator<Item = T>,
    T: Borrow<PreviewTurn>,
{
    let mut normalized = turns
        .into_iter()
        .filter_map(|turn| {
            let turn = turn.borrow();
            let question = if normalize_codex_prompts {
                crate::preview_source::codex::normalize_codex_user_text(&turn.question, None)
            } else {
                turn.question.trim().to_string()
            };
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

fn normalize_cached_codex_prompt(value: Option<String>, normalize_codex: bool) -> Option<String> {
    value.and_then(|text| {
        let normalized = if normalize_codex {
            crate::preview_source::codex::normalize_codex_user_text(&text, None)
        } else {
            text.trim().to_string()
        };
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::normalize_turns;
    use crate::model::PreviewTurn;

    #[test]
    fn normalize_turns_matches_for_owned_and_borrowed_inputs() {
        let turns = vec![
            PreviewTurn {
                question: "  hello  ".to_string(),
                answer: Some("  world  ".to_string()),
            },
            PreviewTurn {
                question: "   ".to_string(),
                answer: Some("drop".to_string()),
            },
        ];

        let from_owned = normalize_turns(turns.clone(), false);
        let from_borrowed = normalize_turns(&turns, false);

        assert_eq!(from_owned, from_borrowed);
        assert_eq!(
            from_owned,
            vec![PreviewTurn {
                question: "hello".to_string(),
                answer: Some("world".to_string()),
            }]
        );
    }
}
