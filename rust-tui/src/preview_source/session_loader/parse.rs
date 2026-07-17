use super::super::{claude, codex, gemini, grok, opencode, SessionReadMode};
use crate::model::{AgentType, PreviewTurn};
use std::path::Path;

pub(super) fn parse_session_transcript(
    agent_type: &AgentType,
    transcript_path: &Path,
    session_id: Option<&str>,
) -> Result<Vec<PreviewTurn>, String> {
    match agent_type {
        AgentType::Codex => codex::parse_transcript(transcript_path, SessionReadMode::FullBackfill),
        AgentType::Claude => {
            claude::parse_transcript(transcript_path, SessionReadMode::FullBackfill)
        }
        AgentType::Gemini => {
            gemini::parse_transcript(transcript_path, SessionReadMode::FullBackfill)
        }
        AgentType::Grok => grok::parse_transcript(transcript_path, SessionReadMode::FullBackfill),
        AgentType::OpenCode => {
            opencode::parse_transcript(transcript_path, session_id, SessionReadMode::FullBackfill)
        }
        _ => Ok(Vec::new()),
    }
}
