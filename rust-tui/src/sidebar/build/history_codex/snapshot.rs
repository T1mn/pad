use crate::session_cache::SessionCacheSnapshot;
use crate::sidebar::display::clean_title;
use crate::sidebar::model::SidebarThread;

pub(super) fn apply_session_cache_snapshot(
    thread: &mut SidebarThread,
    snapshot: &SessionCacheSnapshot,
) {
    if thread.transcript_path.is_none() {
        thread.transcript_path = snapshot.transcript_path.clone();
    }

    if !snapshot.recent_turns.is_empty() {
        thread.cached_preview_turns = snapshot.recent_turns.clone();
    }

    if let Some(prompt) = cached_prompt(snapshot) {
        thread.last_user_prompt = Some(prompt.clone());
        thread.subtitle = Some(prompt);
    }

    if let Some(answer) = snapshot
        .last_assistant_message
        .as_deref()
        .and_then(clean_title)
    {
        thread.last_assistant_message = Some(answer);
    }

    thread.session_cache_state = Some(snapshot.state);
}

fn cached_prompt(snapshot: &SessionCacheSnapshot) -> Option<String> {
    snapshot
        .last_user_prompt
        .as_deref()
        .and_then(clean_cached_prompt)
        .or_else(|| {
            snapshot
                .recent_turns
                .first()
                .and_then(|turn| clean_cached_prompt(&turn.question))
        })
}

fn clean_cached_prompt(text: &str) -> Option<String> {
    clean_title(&crate::preview_source::codex::normalize_codex_user_text(
        text, None,
    ))
}
