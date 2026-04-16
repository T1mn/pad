use super::codex;
use super::PreviewRequest;
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType, PreviewSessionOrigin};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SessionTarget {
    pub(super) origin: PreviewSessionOrigin,
    pub(super) session_id: Option<String>,
    pub(super) transcript_path: PathBuf,
    pub(super) updated_at: Option<i64>,
}

pub(super) fn resolve_session_target(request: &PreviewRequest) -> Option<SessionTarget> {
    let started_at = Instant::now();
    let gemini_thread = if request.agent_type == AgentType::Gemini {
        gemini_thread_for_request(request)
    } else {
        None
    };
    let resolved_session_id = resolved_session_id_for_request(request, gemini_thread.as_ref());
    let claude_thread = if request.agent_type == AgentType::Claude {
        resolved_session_id
            .as_deref()
            .and_then(claude_thread_for_session_id)
    } else {
        None
    };
    let transcript_path = resolve_transcript_path(
        request,
        &PreviewRequest {
            agent_session_id: resolved_session_id.clone(),
            ..request.clone()
        },
        claude_thread.as_ref(),
        gemini_thread.as_ref(),
    )?;
    let updated_at = match request.agent_type {
        AgentType::Codex => resolved_session_id
            .as_deref()
            .and_then(|session_id| crate::codex_state::thread_for_id(session_id).ok().flatten())
            .map(|thread| thread.updated_at)
            .or_else(|| transcript_updated_at(&transcript_path)),
        AgentType::Claude => claude_thread
            .as_ref()
            .map(|thread| thread.updated_at)
            .or_else(|| transcript_updated_at(&transcript_path)),
        AgentType::Gemini => gemini_thread
            .as_ref()
            .map(|thread| thread.updated_at)
            .or_else(|| transcript_updated_at(&transcript_path)),
        _ => transcript_updated_at(&transcript_path),
    };

    Some(SessionTarget {
        origin: request.session_origin.unwrap_or(PreviewSessionOrigin::Pane),
        session_id: resolved_session_id,
        transcript_path,
        updated_at,
    })
    .inspect(|target| {
        if started_at.elapsed().as_millis() >= 15 {
            crate::log_debug!(
                "session_target.resolve: target={} agent={} elapsed_ms={} path={}",
                request.target_key,
                request.agent_type,
                started_at.elapsed().as_millis(),
                target.transcript_path.display()
            );
        }
    })
}

pub(super) fn persistence_panel_from_request(
    request: &PreviewRequest,
    target: &SessionTarget,
) -> Option<AgentPanel> {
    let pane_id = request.live_pane_id.clone()?;
    Some(AgentPanel {
        session: String::new(),
        window: String::new(),
        window_index: String::new(),
        pane: String::new(),
        pane_id,
        agent_type: request.agent_type.clone(),
        working_dir: request.working_dir.clone(),
        is_active: matches!(request.state, AgentState::Busy | AgentState::Waiting),
        state: request.state.clone(),
        state_source: AgentStateSource::Scanner,
        transcript_path: Some(target.transcript_path.to_string_lossy().to_string()),
        cached_preview_turns: request.cached_preview_turns.clone(),
        session_cache_state: request.session_cache_state,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: target.session_id.clone(),
        last_user_prompt: request
            .cached_preview_turns
            .first()
            .map(|turn| turn.question.clone()),
        last_assistant_message: request
            .cached_preview_turns
            .first()
            .and_then(|turn| turn.answer.clone()),
        has_unread_stop: false,
    })
}

fn resolve_transcript_path(
    original_request: &PreviewRequest,
    request: &PreviewRequest,
    claude_thread: Option<&crate::claude_history::ClaudeThreadRef>,
    gemini_thread: Option<&crate::gemini_history::GeminiThreadRef>,
) -> Option<PathBuf> {
    if let Some(path) = request.transcript_path.as_ref() {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let session_id = request.agent_session_id.as_deref()?;
    match request.agent_type {
        AgentType::Codex => codex_transcript_path_for_session_id(session_id).or_else(|| {
            find_matching_jsonl(&dirs::home_dir()?.join(".codex").join("sessions"), |name| {
                name.ends_with(".jsonl") && name.contains(session_id)
            })
        }),
        AgentType::Claude => {
            claude_transcript_path_for_session_id_from_thread(session_id, claude_thread).or_else(|| {
                    let started_at = Instant::now();
                    let expected = format!("{}.jsonl", session_id);
                    let path = find_matching_jsonl(
                        &dirs::home_dir()?.join(".claude").join("projects"),
                        |name| name == expected,
                    );
                    if started_at.elapsed().as_millis() >= 15 {
                        crate::log_debug!(
                            "session_target.resolve: target={} agent=claude fallback=filesystem elapsed_ms={} session_id={} hit={}",
                            original_request.target_key,
                            started_at.elapsed().as_millis(),
                            session_id,
                            path.is_some()
                        );
                    }
                    path
                })
        }
        AgentType::Gemini => gemini_transcript_path_for_session_id_from_thread(session_id, gemini_thread),
        _ => None,
    }
}

fn resolved_session_id_for_request(
    request: &PreviewRequest,
    gemini_thread: Option<&crate::gemini_history::GeminiThreadRef>,
) -> Option<String> {
    if request.agent_type == AgentType::Gemini {
        return request
            .agent_session_id
            .clone()
            .or_else(|| gemini_thread.map(|thread| thread.session_id.clone()))
            .or_else(|| {
                request.transcript_path.as_deref().and_then(|path| {
                    super::gemini::extract_session_id_from_transcript(Path::new(path))
                })
            });
    }

    request
        .agent_session_id
        .clone()
        .or_else(|| {
            if request.agent_type == AgentType::Codex {
                codex_thread_for_working_dir(&request.working_dir).map(|thread| thread.thread_id)
            } else {
                None
            }
        })
        .or_else(|| {
            if request.transcript_path.is_some() {
                None
            } else if request.agent_type == AgentType::Codex && request.state == AgentState::Idle {
                request
                    .live_pane_id
                    .as_deref()
                    .and_then(codex::resolve_live_session_id)
            } else {
                None
            }
        })
}

fn codex_thread_for_working_dir(working_dir: &str) -> Option<crate::codex_state::CodexThreadRef> {
    crate::codex_state::latest_thread_for_cwd(Path::new(working_dir))
        .ok()
        .flatten()
}

fn codex_transcript_path_for_session_id(session_id: &str) -> Option<PathBuf> {
    crate::codex_state::thread_for_id(session_id)
        .ok()
        .flatten()
        .map(|thread| thread.rollout_path)
        .filter(|path| path.exists())
}

fn claude_transcript_path_for_session_id_from_thread(
    session_id: &str,
    claude_thread: Option<&crate::claude_history::ClaudeThreadRef>,
) -> Option<PathBuf> {
    let thread = claude_thread?;
    if thread.session_id != session_id {
        return None;
    }
    let transcript_path = thread.transcript_path.clone();
    transcript_path.exists().then_some(transcript_path)
}

fn claude_thread_for_session_id(
    session_id: &str,
) -> Option<crate::claude_history::ClaudeThreadRef> {
    crate::claude_history::thread_for_id(session_id)
        .ok()
        .flatten()
}

fn gemini_thread_for_request(
    request: &PreviewRequest,
) -> Option<crate::gemini_history::GeminiThreadRef> {
    if let Some(session_id) = request.agent_session_id.as_deref() {
        if let Some(thread) = gemini_thread_for_session_id(session_id) {
            return Some(thread);
        }
    }

    if let Some(path) = request.transcript_path.as_deref() {
        if let Some(thread) = gemini_thread_for_transcript_path(Path::new(path)) {
            return Some(thread);
        }
    }

    gemini_thread_for_working_dir(&request.working_dir)
}

fn gemini_thread_for_session_id(
    session_id: &str,
) -> Option<crate::gemini_history::GeminiThreadRef> {
    crate::gemini_history::thread_for_id(session_id)
        .ok()
        .flatten()
}

fn gemini_thread_for_working_dir(
    working_dir: &str,
) -> Option<crate::gemini_history::GeminiThreadRef> {
    let threads = crate::gemini_history::threads_for_cwd(Path::new(working_dir)).ok()?;
    if let Some(thread) = threads.iter().find(|thread| thread.kind == "main").cloned() {
        return Some(thread);
    }
    threads.into_iter().next()
}

fn gemini_thread_for_transcript_path(
    transcript_path: &Path,
) -> Option<crate::gemini_history::GeminiThreadRef> {
    let threads = crate::gemini_history::all_threads().ok()?;
    threads
        .into_iter()
        .find(|thread| same_path(&thread.transcript_path, transcript_path))
}

fn gemini_transcript_path_for_session_id_from_thread(
    session_id: &str,
    gemini_thread: Option<&crate::gemini_history::GeminiThreadRef>,
) -> Option<PathBuf> {
    let thread = gemini_thread?;
    if thread.session_id != session_id {
        return None;
    }
    let transcript_path = thread.transcript_path.clone();
    transcript_path.exists().then_some(transcript_path)
}

fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn transcript_updated_at(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
}

fn find_matching_jsonl<F>(root: &Path, matcher: F) -> Option<PathBuf>
where
    F: Fn(&str) -> bool,
{
    if !root.exists() {
        return None;
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let file_name = path.file_name()?.to_string_lossy();
            if matcher(&file_name) {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{persistence_panel_from_request, resolved_session_id_for_request, SessionTarget};
    use crate::model::{
        AgentState, AgentType, PreviewSessionOrigin, PreviewTurn, SessionCacheState,
    };
    use crate::preview_source::PreviewRequest;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_json_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pad-gemini-target-{}-{}.json", name, stamp))
    }

    fn base_request() -> PreviewRequest {
        PreviewRequest {
            target_key: "gemini:test".into(),
            live_pane_id: Some("%7".into()),
            agent_type: AgentType::Gemini,
            working_dir: "/tmp/gemini".into(),
            state: AgentState::Idle,
            transcript_path: None,
            cached_preview_turns: vec![PreviewTurn {
                question: "hello".into(),
                answer: Some("world".into()),
            }]
            .into(),
            session_cache_state: Some(SessionCacheState::Cached),
            agent_session_id: None,
            session_origin: Some(PreviewSessionOrigin::Pane),
            persist_resolved_session: true,
            known_updated_at: None,
        }
    }

    #[test]
    fn gemini_session_id_can_be_read_from_transcript_path() {
        let path = temp_json_path("session-id");
        fs::write(
            &path,
            concat!(
                "{",
                "\"sessionId\":\"gemini-session-1\",",
                "\"kind\":\"main\",",
                "\"messages\":[]",
                "}"
            ),
        )
        .unwrap();

        let mut request = base_request();
        request.transcript_path = Some(path.to_string_lossy().to_string());

        let session_id = resolved_session_id_for_request(&request, None);
        fs::remove_file(&path).ok();

        assert_eq!(session_id.as_deref(), Some("gemini-session-1"));
    }

    #[test]
    fn persistence_panel_uses_resolved_target_session_id() {
        let request = base_request();
        let target = SessionTarget {
            origin: PreviewSessionOrigin::Pane,
            session_id: Some("gemini-session-2".into()),
            transcript_path: PathBuf::from("/tmp/gemini-session-2.json"),
            updated_at: Some(42),
        };

        let panel = persistence_panel_from_request(&request, &target).unwrap();
        assert_eq!(panel.agent_session_id.as_deref(), Some("gemini-session-2"));
        assert_eq!(
            panel.transcript_path.as_deref(),
            Some("/tmp/gemini-session-2.json")
        );
    }
}
