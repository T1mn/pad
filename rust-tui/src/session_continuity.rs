use crate::hook::HookEvent;
use crate::model::AgentType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const CONTINUITY_VERSION: u32 = 1;
const LAGGING_THRESHOLD_SECS: i64 = 10;
const FROZEN_THRESHOLD_SECS: i64 = 30;

static CONTINUITY_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

mod storage;

use storage::{append_diagnostic, load_record_snapshot, mutate_record};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityHealth {
    #[default]
    Healthy,
    Lagging,
    Frozen,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityAttemptClassification {
    #[default]
    Normal,
    TransientResumeBootstrap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuityWriteSource {
    Hook,
    Resolver,
}

impl ContinuityWriteSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Hook => "hook",
            Self::Resolver => "resolver",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct ContinuityLedger {
    version: u32,
    sessions: Vec<SessionContinuityRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SessionContinuityRecord {
    session_id: String,
    agent_type: Option<String>,
    transcript_path: Option<String>,
    last_hook_event: Option<String>,
    last_turn_id: Option<String>,
    last_hook_event_at: Option<i64>,
    last_prompt_submit_at: Option<i64>,
    last_stop_at: Option<i64>,
    last_assistant_message_at: Option<i64>,
    last_hook_cache_persist_at: Option<i64>,
    last_resolver_sync_at: Option<i64>,
    last_thread_updated_at: Option<i64>,
    last_rollout_seen_at: Option<i64>,
    last_rollout_mtime: Option<i64>,
    last_rollout_size: Option<u64>,
    lag_seconds: Option<i64>,
    stale_event_count: u32,
    bootstrap_event_count: u32,
    health: ContinuityHealth,
    attempt_classification: ContinuityAttemptClassification,
    updated_at: i64,
}

impl SessionContinuityRecord {
    fn new(session_id: &str, now: i64) -> Self {
        Self {
            session_id: session_id.to_string(),
            agent_type: None,
            transcript_path: None,
            last_hook_event: None,
            last_turn_id: None,
            last_hook_event_at: None,
            last_prompt_submit_at: None,
            last_stop_at: None,
            last_assistant_message_at: None,
            last_hook_cache_persist_at: None,
            last_resolver_sync_at: None,
            last_thread_updated_at: None,
            last_rollout_seen_at: None,
            last_rollout_mtime: None,
            last_rollout_size: None,
            lag_seconds: None,
            stale_event_count: 0,
            bootstrap_event_count: 0,
            health: ContinuityHealth::Healthy,
            attempt_classification: ContinuityAttemptClassification::Normal,
            updated_at: now,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreviewFallbackDecision {
    pub prefer_cache: bool,
    pub health: ContinuityHealth,
    pub attempt_classification: ContinuityAttemptClassification,
    pub lag_seconds: Option<i64>,
    pub reason: &'static str,
}

pub struct PreviewFallbackInput<'a> {
    pub agent_type: &'a AgentType,
    pub session_id: Option<&'a str>,
    pub transcript_path: Option<&'a Path>,
    pub transcript_updated_at: Option<i64>,
    pub thread_updated_at: Option<i64>,
    pub known_updated_at: Option<i64>,
    pub cached_turn_count: usize,
    pub transcript_turn_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuitySnapshot {
    pub session_id: String,
    pub agent_type: Option<String>,
    pub transcript_path: Option<String>,
    pub last_hook_event: Option<String>,
    pub last_turn_id: Option<String>,
    pub last_hook_event_at: Option<i64>,
    pub last_prompt_submit_at: Option<i64>,
    pub last_stop_at: Option<i64>,
    pub last_assistant_message_at: Option<i64>,
    pub last_hook_cache_persist_at: Option<i64>,
    pub last_resolver_sync_at: Option<i64>,
    pub last_thread_updated_at: Option<i64>,
    pub last_rollout_seen_at: Option<i64>,
    pub last_rollout_mtime: Option<i64>,
    pub last_rollout_size: Option<u64>,
    pub lag_seconds: Option<i64>,
    pub stale_event_count: u32,
    pub bootstrap_event_count: u32,
    pub health: ContinuityHealth,
    pub attempt_classification: ContinuityAttemptClassification,
    pub updated_at: i64,
}

impl From<SessionContinuityRecord> for ContinuitySnapshot {
    fn from(record: SessionContinuityRecord) -> Self {
        Self {
            session_id: record.session_id,
            agent_type: record.agent_type,
            transcript_path: record.transcript_path,
            last_hook_event: record.last_hook_event,
            last_turn_id: record.last_turn_id,
            last_hook_event_at: record.last_hook_event_at,
            last_prompt_submit_at: record.last_prompt_submit_at,
            last_stop_at: record.last_stop_at,
            last_assistant_message_at: record.last_assistant_message_at,
            last_hook_cache_persist_at: record.last_hook_cache_persist_at,
            last_resolver_sync_at: record.last_resolver_sync_at,
            last_thread_updated_at: record.last_thread_updated_at,
            last_rollout_seen_at: record.last_rollout_seen_at,
            last_rollout_mtime: record.last_rollout_mtime,
            last_rollout_size: record.last_rollout_size,
            lag_seconds: record.lag_seconds,
            stale_event_count: record.stale_event_count,
            bootstrap_event_count: record.bootstrap_event_count,
            health: record.health,
            attempt_classification: record.attempt_classification,
            updated_at: record.updated_at,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct ContinuityDiagnosticEvent {
    ts: i64,
    kind: &'static str,
    session_id: String,
    agent_type: Option<String>,
    event: Option<String>,
    turn_id: Option<String>,
    transcript_path: Option<String>,
    source: Option<&'static str>,
    health: ContinuityHealth,
    attempt_classification: ContinuityAttemptClassification,
    lag_seconds: Option<i64>,
    stale_event_count: u32,
    rollout_mtime: Option<i64>,
    rollout_size: Option<u64>,
    thread_updated_at: Option<i64>,
    cached_turns: Option<usize>,
    transcript_turns: Option<usize>,
    prefer_cache: Option<bool>,
    reason: Option<&'static str>,
}

pub fn record_hook_event(
    agent_type: Option<&AgentType>,
    event: &HookEvent,
    fallback_session_id: Option<&str>,
    fallback_transcript_path: Option<&str>,
) {
    let Some(session_id) = clean_text(event.session_id.as_deref().or(fallback_session_id)) else {
        return;
    };
    let transcript_path = clean_text(
        event
            .transcript_path
            .as_deref()
            .or(fallback_transcript_path),
    )
    .map(str::to_owned);
    let now = now_ts();

    let record = mutate_record(session_id, now, |record| {
        if let Some(agent_type) = agent_type {
            record.agent_type = Some(agent_type.to_string());
        }
        record.last_hook_event = Some(event.event.clone());
        record.last_hook_event_at = Some(now);
        record.last_turn_id = clean_text(event.turn_id.as_deref()).map(str::to_owned);
        record.updated_at = now;

        match event.event.as_str() {
            "session_start" if transcript_path.is_none() => {
                record.attempt_classification =
                    ContinuityAttemptClassification::TransientResumeBootstrap;
                record.bootstrap_event_count = record.bootstrap_event_count.saturating_add(1);
            }
            "user_prompt_submit" => {
                record.last_prompt_submit_at = Some(now);
            }
            "stop" => {
                record.last_stop_at = Some(now);
                if clean_text(event.last_assistant_message.as_deref()).is_some() {
                    record.last_assistant_message_at = Some(now);
                }
            }
            _ => {}
        }

        observe_transcript(record, transcript_path.as_deref().map(Path::new), now);
        clear_bootstrap_if_resolved(record);
        recompute_record_health(record);
    });

    if let Some(record) = record {
        append_diagnostic(&ContinuityDiagnosticEvent {
            ts: now,
            kind: "hook_event",
            session_id: record.session_id.clone(),
            agent_type: record.agent_type.clone(),
            event: Some(event.event.clone()),
            turn_id: record.last_turn_id.clone(),
            transcript_path: record.transcript_path.clone(),
            source: None,
            health: record.health,
            attempt_classification: record.attempt_classification,
            lag_seconds: record.lag_seconds,
            stale_event_count: record.stale_event_count,
            rollout_mtime: record.last_rollout_mtime,
            rollout_size: record.last_rollout_size,
            thread_updated_at: record.last_thread_updated_at,
            cached_turns: None,
            transcript_turns: None,
            prefer_cache: None,
            reason: None,
        });
    }
}

pub fn record_cache_write(
    agent_type: &AgentType,
    session_id: &str,
    transcript_path: Option<&Path>,
    source: ContinuityWriteSource,
    turn_count: usize,
) {
    let now = now_ts();
    let record = mutate_record(session_id, now, |record| {
        record.agent_type = Some(agent_type.to_string());
        record.updated_at = now;
        match source {
            ContinuityWriteSource::Hook => {
                record.last_hook_cache_persist_at = Some(now);
            }
            ContinuityWriteSource::Resolver => {
                record.last_resolver_sync_at = Some(now);
            }
        }
        observe_transcript(record, transcript_path, now);
        clear_bootstrap_if_resolved(record);
        recompute_record_health(record);
    });

    if let Some(record) = record {
        append_diagnostic(&ContinuityDiagnosticEvent {
            ts: now,
            kind: "cache_write",
            session_id: record.session_id.clone(),
            agent_type: record.agent_type.clone(),
            event: None,
            turn_id: record.last_turn_id.clone(),
            transcript_path: record.transcript_path.clone(),
            source: Some(source.as_str()),
            health: record.health,
            attempt_classification: record.attempt_classification,
            lag_seconds: record.lag_seconds,
            stale_event_count: record.stale_event_count,
            rollout_mtime: record.last_rollout_mtime,
            rollout_size: record.last_rollout_size,
            thread_updated_at: record.last_thread_updated_at,
            cached_turns: Some(turn_count),
            transcript_turns: None,
            prefer_cache: None,
            reason: None,
        });
    }
}

pub fn assess_preview_fallback(input: PreviewFallbackInput<'_>) -> Option<PreviewFallbackDecision> {
    if input.cached_turn_count == 0 {
        return None;
    }
    let session_id = clean_text(input.session_id)?;
    let snapshot = load_record_snapshot(session_id)?;
    let runtime_activity_at = max_ts(
        snapshot.last_hook_event_at,
        max_ts(
            snapshot.last_hook_cache_persist_at,
            max_ts(input.thread_updated_at, input.known_updated_at),
        ),
    );
    let rollout_mtime = input.transcript_updated_at.or(snapshot.last_rollout_mtime);
    let lag_seconds = lag_seconds(runtime_activity_at, rollout_mtime);
    let health = classify_preview_health(
        lag_seconds,
        snapshot.stale_event_count,
        input.thread_updated_at,
        input.known_updated_at,
    );
    let prefer_cache = health == ContinuityHealth::Frozen && input.transcript_turn_count > 0;
    let reason = if prefer_cache {
        "rollout_frozen"
    } else if health == ContinuityHealth::Lagging {
        "rollout_lagging"
    } else if snapshot.attempt_classification
        == ContinuityAttemptClassification::TransientResumeBootstrap
    {
        "transient_resume_bootstrap"
    } else {
        "healthy"
    };

    if health == ContinuityHealth::Healthy
        && snapshot.attempt_classification == ContinuityAttemptClassification::Normal
    {
        return None;
    }

    if !matches!(
        input.agent_type,
        AgentType::Codex | AgentType::Claude | AgentType::Gemini
    ) {
        return None;
    }

    let _ = input.transcript_path;

    Some(PreviewFallbackDecision {
        prefer_cache,
        health,
        attempt_classification: snapshot.attempt_classification,
        lag_seconds,
        reason,
    })
}

pub fn record_preview_assessment(
    agent_type: &AgentType,
    session_id: Option<&str>,
    transcript_path: Option<&Path>,
    thread_updated_at: Option<i64>,
    cached_turn_count: usize,
    transcript_turn_count: usize,
    decision: &PreviewFallbackDecision,
) {
    let Some(session_id) = clean_text(session_id) else {
        return;
    };
    let now = now_ts();
    let record = mutate_record(session_id, now, |record| {
        record.agent_type = Some(agent_type.to_string());
        record.updated_at = now;
        if let Some(thread_updated_at) = thread_updated_at {
            record.last_thread_updated_at = Some(match record.last_thread_updated_at {
                Some(existing) => existing.max(thread_updated_at),
                None => thread_updated_at,
            });
        }
        observe_transcript(record, transcript_path, now);
        recompute_record_health(record);
    });

    if let Some(record) = record {
        append_diagnostic(&ContinuityDiagnosticEvent {
            ts: now,
            kind: "preview_assessment",
            session_id: record.session_id.clone(),
            agent_type: record.agent_type.clone(),
            event: None,
            turn_id: record.last_turn_id.clone(),
            transcript_path: record.transcript_path.clone(),
            source: None,
            health: decision.health,
            attempt_classification: decision.attempt_classification,
            lag_seconds: decision.lag_seconds,
            stale_event_count: record.stale_event_count,
            rollout_mtime: record.last_rollout_mtime,
            rollout_size: record.last_rollout_size,
            thread_updated_at: record.last_thread_updated_at,
            cached_turns: Some(cached_turn_count),
            transcript_turns: Some(transcript_turn_count),
            prefer_cache: Some(decision.prefer_cache),
            reason: Some(decision.reason),
        });
    }
}

pub fn load_snapshot_for(
    session_id: Option<&str>,
    transcript_path: Option<&str>,
) -> Option<ContinuitySnapshot> {
    storage::load_snapshot_for(session_id, transcript_path)
}

fn observe_transcript(
    record: &mut SessionContinuityRecord,
    transcript_path: Option<&Path>,
    now: i64,
) {
    let Some(transcript_path) = transcript_path else {
        return;
    };
    record.transcript_path = Some(transcript_path.to_string_lossy().to_string());
    record.last_rollout_seen_at = Some(now);

    let Ok(metadata) = fs::metadata(transcript_path) else {
        return;
    };
    record.last_rollout_size = Some(metadata.len());
    record.last_rollout_mtime = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64);
}

fn clear_bootstrap_if_resolved(record: &mut SessionContinuityRecord) {
    if record.transcript_path.is_some()
        || record.last_hook_cache_persist_at.is_some()
        || record.last_resolver_sync_at.is_some()
    {
        record.attempt_classification = ContinuityAttemptClassification::Normal;
    }
}

fn recompute_record_health(record: &mut SessionContinuityRecord) {
    let runtime_activity_at = max_ts(record.last_hook_event_at, record.last_hook_cache_persist_at);
    let lag_seconds = lag_seconds(runtime_activity_at, record.last_rollout_mtime);
    record.stale_event_count = next_stale_event_count(record.stale_event_count, lag_seconds);
    record.lag_seconds = lag_seconds;
    record.health = classify_health(lag_seconds, record.stale_event_count);
}

fn lag_seconds(runtime_activity_at: Option<i64>, rollout_mtime: Option<i64>) -> Option<i64> {
    let runtime_activity_at = runtime_activity_at?;
    let rollout_mtime = rollout_mtime?;
    (runtime_activity_at > rollout_mtime).then_some(runtime_activity_at - rollout_mtime)
}

fn next_stale_event_count(current: u32, lag_seconds: Option<i64>) -> u32 {
    match lag_seconds {
        Some(lag_seconds) if lag_seconds >= LAGGING_THRESHOLD_SECS => {
            current.saturating_add(1).max(1)
        }
        _ => 0,
    }
}

fn classify_health(lag_seconds: Option<i64>, stale_event_count: u32) -> ContinuityHealth {
    match lag_seconds {
        Some(lag_seconds) if lag_seconds >= FROZEN_THRESHOLD_SECS && stale_event_count >= 2 => {
            ContinuityHealth::Frozen
        }
        Some(lag_seconds) if lag_seconds >= LAGGING_THRESHOLD_SECS => ContinuityHealth::Lagging,
        _ => ContinuityHealth::Healthy,
    }
}

fn classify_preview_health(
    lag_seconds: Option<i64>,
    stale_event_count: u32,
    thread_updated_at: Option<i64>,
    known_updated_at: Option<i64>,
) -> ContinuityHealth {
    match lag_seconds {
        Some(lag_seconds)
            if lag_seconds >= FROZEN_THRESHOLD_SECS
                && (stale_event_count >= 2
                    || thread_updated_at.is_some()
                    || known_updated_at.is_some()) =>
        {
            ContinuityHealth::Frozen
        }
        Some(lag_seconds) if lag_seconds >= LAGGING_THRESHOLD_SECS => ContinuityHealth::Lagging,
        _ => ContinuityHealth::Healthy,
    }
}

fn clean_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| !text.is_empty())
}

fn max_ts(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
