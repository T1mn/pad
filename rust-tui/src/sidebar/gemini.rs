use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const ACTIVE_THREAD_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeminiThreadRef {
    pub session_id: String,
    pub project_root: PathBuf,
    pub updated_at: i64,
    pub title: Option<String>,
    pub first_user_prompt: Option<String>,
    pub last_user_prompt: Option<String>,
    pub kind: String,
    pub archived: bool,
}

pub fn all_threads() -> io::Result<Vec<GeminiThreadRef>> {
    Ok(crate::gemini_history::all_threads()?
        .into_iter()
        .filter(is_visible_thread)
        .filter(is_recent_thread)
        .map(map_thread_ref)
        .collect())
}

pub fn all_archived_threads() -> io::Result<Vec<GeminiThreadRef>> {
    Ok(crate::gemini_history::all_archived_threads()?
        .into_iter()
        .filter(is_visible_thread)
        .map(map_thread_ref)
        .collect())
}

pub fn threads_for_cwd(cwd: &Path) -> io::Result<Vec<GeminiThreadRef>> {
    let normalized = normalize_path(cwd);
    Ok(crate::gemini_history::threads_for_cwd(cwd)?
        .into_iter()
        .filter(is_visible_thread)
        .filter(is_recent_thread)
        .filter(|thread| normalize_path(&thread.cwd) == normalized)
        .map(map_thread_ref)
        .collect())
}

pub fn archived_threads_for_cwd(cwd: &Path) -> io::Result<Vec<GeminiThreadRef>> {
    let normalized = normalize_path(cwd);
    Ok(crate::gemini_history::all_archived_threads()?
        .into_iter()
        .filter(is_visible_thread)
        .filter(|thread| normalize_path(&thread.cwd) == normalized)
        .map(map_thread_ref)
        .collect())
}

pub fn thread_for_id(session_id: &str) -> io::Result<Option<GeminiThreadRef>> {
    Ok(crate::gemini_history::thread_for_id(session_id)?.map(map_thread_ref))
}

pub fn archive_thread(session_id: &str) -> io::Result<()> {
    crate::gemini_history::archive_thread(session_id)
}

pub fn unarchive_thread(session_id: &str) -> io::Result<()> {
    crate::gemini_history::unarchive_thread(session_id)
}

fn map_thread_ref(thread: crate::gemini_history::GeminiThreadRef) -> GeminiThreadRef {
    GeminiThreadRef {
        session_id: thread.session_id,
        project_root: thread.cwd,
        updated_at: thread.updated_at,
        title: thread
            .title
            .clone()
            .or(thread.summary.clone())
            .or(thread.first_user_message.clone()),
        first_user_prompt: thread.first_user_message,
        last_user_prompt: thread.last_user_message.or(thread.subtitle),
        kind: thread.kind,
        archived: thread.archived,
    }
}

fn is_visible_thread(thread: &crate::gemini_history::GeminiThreadRef) -> bool {
    thread.kind.trim() != "subagent"
}

fn is_recent_thread(thread: &crate::gemini_history::GeminiThreadRef) -> bool {
    thread.updated_at >= active_cutoff_ts()
}

fn active_cutoff_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
        .saturating_sub(ACTIVE_THREAD_MAX_AGE_SECS)
}

fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_filter_hides_subagent_threads() {
        let thread = crate::gemini_history::GeminiThreadRef {
            session_id: "abc".into(),
            cwd: PathBuf::from("/tmp/demo"),
            updated_at: 1,
            transcript_path: PathBuf::from("/tmp/demo/session.json"),
            title: Some("hello".into()),
            subtitle: Some("prompt".into()),
            first_user_message: Some("hello".into()),
            last_user_message: Some("prompt".into()),
            last_assistant_message: None,
            summary: None,
            kind: "subagent".into(),
            archived: false,
            has_subagent: true,
        };
        assert!(!is_visible_thread(&thread));
    }
}
