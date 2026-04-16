use crate::hook::HookEvent;
use crate::model::AgentType;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const CONTINUITY_VERSION: u32 = 1;
const LAGGING_THRESHOLD_SECS: i64 = 10;
const FROZEN_THRESHOLD_SECS: i64 = 30;

static CONTINUITY_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

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

pub fn assess_preview_fallback(
    agent_type: &AgentType,
    session_id: Option<&str>,
    transcript_path: Option<&Path>,
    transcript_updated_at: Option<i64>,
    thread_updated_at: Option<i64>,
    known_updated_at: Option<i64>,
    cached_turn_count: usize,
    transcript_turn_count: usize,
) -> Option<PreviewFallbackDecision> {
    if cached_turn_count == 0 {
        return None;
    }
    let session_id = clean_text(session_id)?;
    let snapshot = load_record_snapshot(session_id)?;
    let runtime_activity_at = max_ts(
        snapshot.last_hook_event_at,
        max_ts(
            snapshot.last_hook_cache_persist_at,
            max_ts(thread_updated_at, known_updated_at),
        ),
    );
    let rollout_mtime = transcript_updated_at.or(snapshot.last_rollout_mtime);
    let lag_seconds = lag_seconds(runtime_activity_at, rollout_mtime);
    let health = classify_preview_health(
        lag_seconds,
        snapshot.stale_event_count,
        thread_updated_at,
        known_updated_at,
    );
    let prefer_cache = health == ContinuityHealth::Frozen && transcript_turn_count > 0;
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
        agent_type,
        AgentType::Codex | AgentType::Claude | AgentType::Gemini
    ) {
        return None;
    }

    let _ = transcript_path;

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

pub fn load_snapshot_by_session_id(session_id: &str) -> Option<ContinuitySnapshot> {
    let session_id = clean_text(Some(session_id))?;
    load_record_snapshot(session_id).map(Into::into)
}

pub fn load_snapshot_for(
    session_id: Option<&str>,
    transcript_path: Option<&str>,
) -> Option<ContinuitySnapshot> {
    if let Some(session_id) = session_id.and_then(|value| clean_text(Some(value))) {
        if let Some(snapshot) = load_snapshot_by_session_id(session_id) {
            return Some(snapshot);
        }
    }

    let transcript_path = transcript_path.and_then(|value| clean_text(Some(value)))?;
    let _guard = CONTINUITY_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .ok()?;
    let ledger = load_ledger();
    ledger
        .sessions
        .into_iter()
        .find(|record| {
            record
                .transcript_path
                .as_deref()
                .map(|path| same_path_str(path, transcript_path))
                .unwrap_or(false)
        })
        .map(Into::into)
}

fn mutate_record<F>(session_id: &str, now: i64, mut f: F) -> Option<SessionContinuityRecord>
where
    F: FnMut(&mut SessionContinuityRecord),
{
    let _guard = CONTINUITY_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .ok()?;
    let mut ledger = load_ledger();
    let snapshot = {
        let record = upsert_record(&mut ledger, session_id, now);
        f(record);
        record.clone()
    };
    if let Err(err) = save_ledger(&ledger) {
        crate::log_debug!(
            "session_continuity: failed to save ledger session_id={} err={}",
            session_id,
            err
        );
    }
    Some(snapshot)
}

fn load_record_snapshot(session_id: &str) -> Option<SessionContinuityRecord> {
    let _guard = CONTINUITY_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .ok()?;
    let ledger = load_ledger();
    ledger
        .sessions
        .into_iter()
        .find(|record| record.session_id == session_id)
}

fn load_ledger() -> ContinuityLedger {
    let path = crate::paths::session_continuity_state_path();
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => {
            return ContinuityLedger {
                version: CONTINUITY_VERSION,
                ..ContinuityLedger::default()
            };
        }
    };

    serde_json::from_str(&content).unwrap_or_else(|err| {
        crate::log_debug!(
            "session_continuity: failed to parse {}: {}",
            path.display(),
            err
        );
        ContinuityLedger {
            version: CONTINUITY_VERSION,
            ..ContinuityLedger::default()
        }
    })
}

fn save_ledger(ledger: &ContinuityLedger) -> std::io::Result<()> {
    let path = crate::paths::session_continuity_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}.{}", std::process::id(), now_ts()));
    fs::write(&tmp_path, serde_json::to_string_pretty(ledger)?)?;
    fs::rename(&tmp_path, &path)
}

fn upsert_record<'a>(
    ledger: &'a mut ContinuityLedger,
    session_id: &str,
    now: i64,
) -> &'a mut SessionContinuityRecord {
    ledger.version = CONTINUITY_VERSION;
    if let Some(index) = ledger
        .sessions
        .iter()
        .position(|record| record.session_id == session_id)
    {
        return &mut ledger.sessions[index];
    }

    ledger
        .sessions
        .push(SessionContinuityRecord::new(session_id, now));
    ledger
        .sessions
        .last_mut()
        .expect("session continuity record")
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

fn append_diagnostic(event: &ContinuityDiagnosticEvent) {
    let _guard = match CONTINUITY_IO_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    let path = crate::paths::session_continuity_log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            if let Ok(line) = serde_json::to_string(event) {
                let _ = writeln!(file, "{}", line);
            }
        }
        Err(err) => {
            crate::log_debug!(
                "session_continuity: failed to append diagnostic err={}",
                err
            );
        }
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

fn same_path_str(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }

    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_health, classify_preview_health, recompute_record_health,
        ContinuityAttemptClassification, ContinuityHealth, PreviewFallbackDecision,
        SessionContinuityRecord,
    };

    fn record(session_id: &str) -> SessionContinuityRecord {
        SessionContinuityRecord::new(session_id, 100)
    }

    #[test]
    fn record_becomes_frozen_after_repeated_stale_runtime_activity() {
        let mut record = record("session-1");
        record.last_rollout_mtime = Some(100);
        record.last_hook_event_at = Some(131);
        recompute_record_health(&mut record);
        assert_eq!(record.health, ContinuityHealth::Lagging);
        assert_eq!(record.stale_event_count, 1);

        record.last_hook_cache_persist_at = Some(132);
        recompute_record_health(&mut record);
        assert_eq!(record.health, ContinuityHealth::Frozen);
        assert!(record.lag_seconds.unwrap_or_default() >= 32);
        assert!(record.stale_event_count >= 2);
    }

    #[test]
    fn preview_health_promotes_to_frozen_with_strong_runtime_signal() {
        assert_eq!(
            classify_preview_health(Some(35), 1, Some(140), None),
            ContinuityHealth::Frozen
        );
        assert_eq!(
            classify_preview_health(Some(12), 1, None, None),
            ContinuityHealth::Lagging
        );
        assert_eq!(classify_health(Some(5), 0), ContinuityHealth::Healthy);
    }

    #[test]
    fn bootstrap_classification_clears_once_transcript_is_known() {
        let mut record = record("session-2");
        record.attempt_classification = ContinuityAttemptClassification::TransientResumeBootstrap;
        record.transcript_path = Some("/tmp/demo.jsonl".into());
        super::clear_bootstrap_if_resolved(&mut record);
        assert_eq!(
            record.attempt_classification,
            ContinuityAttemptClassification::Normal
        );
    }

    #[test]
    fn frozen_decision_marks_cache_fallback() {
        let decision = PreviewFallbackDecision {
            prefer_cache: true,
            health: ContinuityHealth::Frozen,
            attempt_classification: ContinuityAttemptClassification::Normal,
            lag_seconds: Some(45),
            reason: "rollout_frozen",
        };
        assert!(decision.prefer_cache);
        assert_eq!(decision.health, ContinuityHealth::Frozen);
        assert_eq!(decision.reason, "rollout_frozen");
    }
}
