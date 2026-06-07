use crate::model::AgentType;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Hook => "hook",
            Self::Resolver => "resolver",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct ContinuityLedger {
    pub(super) version: u32,
    pub(super) sessions: Vec<SessionContinuityRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct SessionContinuityRecord {
    pub(super) session_id: String,
    pub(super) agent_type: Option<String>,
    pub(super) transcript_path: Option<String>,
    pub(super) last_hook_event: Option<String>,
    pub(super) last_turn_id: Option<String>,
    pub(super) last_hook_event_at: Option<i64>,
    pub(super) last_prompt_submit_at: Option<i64>,
    pub(super) last_stop_at: Option<i64>,
    pub(super) last_assistant_message_at: Option<i64>,
    pub(super) last_hook_cache_persist_at: Option<i64>,
    pub(super) last_resolver_sync_at: Option<i64>,
    pub(super) last_thread_updated_at: Option<i64>,
    pub(super) last_rollout_seen_at: Option<i64>,
    pub(super) last_rollout_mtime: Option<i64>,
    pub(super) last_rollout_size: Option<u64>,
    pub(super) lag_seconds: Option<i64>,
    pub(super) stale_event_count: u32,
    pub(super) bootstrap_event_count: u32,
    pub(super) health: ContinuityHealth,
    pub(super) attempt_classification: ContinuityAttemptClassification,
    pub(super) updated_at: i64,
}

impl SessionContinuityRecord {
    pub(super) fn new(session_id: &str, now: i64) -> Self {
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
pub(super) struct ContinuityDiagnosticEvent {
    pub(super) ts: i64,
    pub(super) kind: &'static str,
    pub(super) session_id: String,
    pub(super) agent_type: Option<String>,
    pub(super) event: Option<String>,
    pub(super) turn_id: Option<String>,
    pub(super) transcript_path: Option<String>,
    pub(super) source: Option<&'static str>,
    pub(super) health: ContinuityHealth,
    pub(super) attempt_classification: ContinuityAttemptClassification,
    pub(super) lag_seconds: Option<i64>,
    pub(super) stale_event_count: u32,
    pub(super) rollout_mtime: Option<i64>,
    pub(super) rollout_size: Option<u64>,
    pub(super) thread_updated_at: Option<i64>,
    pub(super) cached_turns: Option<usize>,
    pub(super) transcript_turns: Option<usize>,
    pub(super) prefer_cache: Option<bool>,
    pub(super) reason: Option<&'static str>,
}
