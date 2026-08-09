mod claude {
    use std::path::PathBuf;

    pub(in crate::preview_source::session_target) fn claude_thread_for_session_id(
        session_id: &str,
    ) -> Option<crate::claude_history::ClaudeThreadRef> {
        crate::claude_history::thread_for_id(session_id)
            .ok()
            .flatten()
    }

    pub(in crate::preview_source::session_target) fn claude_transcript_path_for_session_id_from_thread(
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
}
mod codex {
    use std::path::{Path, PathBuf};

    pub(super) fn codex_thread_for_working_dir(
        working_dir: &str,
        require_unique: bool,
    ) -> Option<crate::codex_state::CodexThreadRef> {
        if !require_unique {
            return crate::codex_state::latest_thread_for_cwd(Path::new(working_dir))
                .ok()
                .flatten();
        }
        let threads = crate::codex_state::threads_for_cwd(Path::new(working_dir)).ok()?;
        super::path::select_cwd_candidate(threads, require_unique)
    }

    pub(in crate::preview_source::session_target) fn codex_transcript_path_for_session_id(
        session_id: &str,
    ) -> Option<PathBuf> {
        crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()
            .map(|thread| thread.rollout_path)
            .and_then(|path| crate::codex_rollout::existing_rollout_path(&path))
    }
}
mod gemini {
    use super::path::same_path;
    use crate::preview_source::PreviewRequest;
    use std::path::{Path, PathBuf};

    pub(in crate::preview_source::session_target) fn gemini_thread_for_request(
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

    pub(super) fn gemini_thread_for_session_id(
        session_id: &str,
    ) -> Option<crate::gemini_history::GeminiThreadRef> {
        crate::gemini_history::thread_for_id(session_id)
            .ok()
            .flatten()
    }

    pub(super) fn gemini_thread_for_working_dir(
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

    pub(in crate::preview_source::session_target) fn gemini_transcript_path_for_session_id_from_thread(
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
}
mod grok {
    use std::path::PathBuf;

    pub(in crate::preview_source::session_target) fn grok_transcript_path_for_session_id(
        session_id: &str,
    ) -> Option<PathBuf> {
        crate::grok_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .map(|thread| thread.transcript_path)
    }
}
mod opencode {
    use std::path::Path;

    pub(super) fn opencode_thread_for_working_dir(
        working_dir: &str,
        require_unique: bool,
    ) -> Option<crate::opencode_history::OpenCodeThreadRef> {
        let threads = crate::opencode_history::threads_for_cwd(Path::new(working_dir)).ok()?;
        super::path::select_cwd_candidate(threads, require_unique)
    }
}
mod path {
    use std::fs;
    use std::path::{Path, PathBuf};

    pub(super) fn select_cwd_candidate<T>(threads: Vec<T>, require_unique: bool) -> Option<T> {
        if require_unique && threads.len() != 1 {
            return None;
        }
        threads.into_iter().next()
    }

    pub(super) fn same_path(left: &Path, right: &Path) -> bool {
        if left == right {
            return true;
        }

        match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
            (Ok(left), Ok(right)) => left == right,
            _ => false,
        }
    }

    pub(crate) fn transcript_updated_at(path: &Path) -> Option<i64> {
        std::fs::metadata(path)
            .ok()?
            .modified()
            .ok()
            .and_then(crate::time::system_time_unix_secs)
    }

    pub(in crate::preview_source::session_target) fn find_matching_jsonl<F>(
        root: &Path,
        matcher: F,
    ) -> Option<PathBuf>
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
}
mod resolved {
    use super::codex::codex_thread_for_working_dir;
    use super::opencode::opencode_thread_for_working_dir;
    use crate::model::AgentType;
    use crate::preview_source::PreviewRequest;
    use std::path::Path;

    pub(crate) fn resolved_session_id_for_request(
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
                        super::super::super::gemini::extract_session_id_from_transcript(Path::new(
                            path,
                        ))
                    })
                });
        }

        request
            .agent_session_id
            .clone()
            .or_else(|| {
                if request.agent_type == AgentType::Grok {
                    request
                        .transcript_path
                        .as_deref()
                        .and_then(|path| Path::new(path).parent())
                        .and_then(Path::file_name)
                        .and_then(|name| name.to_str())
                        .map(str::to_string)
                } else {
                    None
                }
            })
            .or_else(|| {
                let require_unique = request.live_pane_id.is_some();
                if request.agent_type == AgentType::Codex {
                    codex_thread_for_working_dir(&request.working_dir, require_unique)
                        .map(|thread| thread.thread_id)
                } else if request.agent_type == AgentType::OpenCode {
                    opencode_thread_for_working_dir(&request.working_dir, require_unique)
                        .map(|thread| thread.session_id)
                } else {
                    None
                }
            })
    }
}

pub(super) use claude::{
    claude_thread_for_session_id, claude_transcript_path_for_session_id_from_thread,
};
pub(super) use codex::codex_transcript_path_for_session_id;
pub(super) use gemini::{
    gemini_thread_for_request, gemini_transcript_path_for_session_id_from_thread,
};
pub(super) use grok::grok_transcript_path_for_session_id;
pub(super) use path::find_matching_jsonl;
pub(crate) use path::transcript_updated_at;
#[cfg(test)]
pub(crate) use resolved::resolved_session_id_for_request;
#[cfg(not(test))]
pub(super) use resolved::resolved_session_id_for_request;

#[cfg(test)]
pub(crate) mod tests {
    pub(crate) mod codex {
        use super::super::codex::codex_transcript_path_for_session_id;

        pub(crate) fn codex_db_canonical_path_resolves_compressed_sibling() {
            crate::test_support::with_temp_home("pad-codex-target", "compressed", |home| {
                let codex_home = home.join(".codex");
                std::fs::create_dir_all(&codex_home).unwrap();
                let canonical = codex_home.join("sessions/rollout-session-zst.jsonl");
                let compressed = canonical.with_extension("jsonl.zst");
                std::fs::create_dir_all(compressed.parent().unwrap()).unwrap();
                std::fs::write(&compressed, b"fixture").unwrap();

                let connection =
                    rusqlite::Connection::open(codex_home.join("state_5.sqlite")).unwrap();
                connection
                    .execute_batch(
                        "CREATE TABLE threads (
                            id TEXT PRIMARY KEY, cwd TEXT NOT NULL, updated_at INTEGER NOT NULL,
                            rollout_path TEXT NOT NULL, title TEXT, first_user_message TEXT,
                            source TEXT, archived INTEGER NOT NULL DEFAULT 0, archived_at INTEGER
                        );",
                    )
                    .unwrap();
                connection
                    .execute(
                        "INSERT INTO threads (id,cwd,updated_at,rollout_path,archived)
                         VALUES (?1,?2,?3,?4,0)",
                        rusqlite::params![
                            "session-zst",
                            "/tmp/project",
                            1_i64,
                            canonical.to_string_lossy().to_string()
                        ],
                    )
                    .unwrap();

                assert_eq!(
                    codex_transcript_path_for_session_id("session-zst"),
                    Some(compressed)
                );
            });
        }
    }

    pub(crate) mod path {
        use super::super::path::select_cwd_candidate;

        pub(crate) fn live_cwd_candidate_requires_one_unambiguous_session() {
            assert_eq!(select_cwd_candidate(vec![1], true), Some(1));
            assert_eq!(select_cwd_candidate(vec![1, 2], true), None);
            assert_eq!(select_cwd_candidate(vec![1, 2], false), Some(1));
        }
    }
}
