use super::enums::{ContinuityAttemptClassification, ContinuityHealth};
use crate::model::AgentType;
use std::path::Path;

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
