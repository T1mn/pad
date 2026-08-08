mod entry {
    use super::snapshot::apply_session_cache_snapshot;
    use crate::codex_state::CodexThreadRef;
    use crate::model::{AgentState, AgentType};
    use crate::session_cache::SessionCacheSnapshot;
    use crate::sidebar::display::{best_thread_title, clean_title};
    use crate::sidebar::model::{SidebarFolder, SidebarThread};

    pub(in crate::sidebar::build) fn build_codex_history_entry(
        folder: &SidebarFolder,
        thread: &CodexThreadRef,
        snapshot: Option<&SessionCacheSnapshot>,
        archived_threads_view: bool,
    ) -> SidebarThread {
        let title = best_thread_title(thread.title.as_deref(), Some(thread.thread_id.as_str()));
        let sort_updated_at = initial_sort_updated_at(thread.updated_at, archived_threads_view);
        let mut history_entry = SidebarThread {
            key: format!("codex:{}", thread.thread_id),
            folder_key: folder.key.clone(),
            working_dir: folder.path.clone(),
            folder_label: folder.label.clone(),
            agent_type: AgentType::Codex,
            session_id: Some(thread.thread_id.clone()),
            transcript_path: Some(thread.rollout_path.to_string_lossy().to_string()),
            session_provider_name: crate::sidebar::provider::resolve_session_provider_name(
                &AgentType::Codex,
                Some(thread.rollout_path.as_path()),
            ),
            title,
            upstream_title: thread.title.as_deref().and_then(clean_title),
            generated_title: None,
            subtitle: None,
            title_override: None,
            note: None,
            share_url: None,
            cost: None,
            token_summary: None,
            tags: Vec::new(),
            pinned: false,
            updated_at: thread.updated_at,
            sort_updated_at,
            live_pane_id: None,
            live_location: None,
            state: AgentState::Idle,
            is_active: false,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
            archived: thread.archived,
            deleted: false,
        };

        if let Some(snapshot) = snapshot {
            apply_session_cache_snapshot(&mut history_entry, snapshot);
        }

        if history_entry.subtitle.is_none() {
            history_entry.subtitle = thread.first_user_message.as_deref().and_then(clean_title);
        }

        history_entry
    }

    fn initial_sort_updated_at(updated_at: i64, archived_threads_view: bool) -> i64 {
        if archived_threads_view {
            updated_at
        } else {
            0
        }
    }
}
mod merge {
    use super::entry::build_codex_history_entry;
    use crate::session_cache::SessionCacheSnapshot;
    use crate::sidebar::build::activity::merge_or_insert_thread;
    use crate::sidebar::model::{SidebarFolder, ThreadActivityOverride};
    use std::collections::HashMap;
    use std::path::Path;

    pub(in crate::sidebar::build) fn merge_codex_threads(
        folder: &mut SidebarFolder,
        activity_overrides: &[ThreadActivityOverride],
        thread_sort_activity: &HashMap<String, i64>,
        codex_session_snapshots: &HashMap<String, SessionCacheSnapshot>,
        archived_threads_view: bool,
    ) -> usize {
        let Ok(threads) = codex_threads_for_folder(folder, archived_threads_view) else {
            return 0;
        };

        let mut merged = 0usize;
        for thread in threads {
            if crate::sidebar::search::is_subagent_source(thread.source.as_deref()) {
                continue;
            }
            let history_entry = build_codex_history_entry(
                folder,
                &thread,
                codex_session_snapshots.get(&thread.thread_id),
                archived_threads_view,
            );

            merge_or_insert_thread(
                &mut folder.threads,
                history_entry,
                activity_overrides,
                thread_sort_activity,
            );
            merged += 1;
        }
        merged
    }

    fn codex_threads_for_folder(
        folder: &SidebarFolder,
        archived_threads_view: bool,
    ) -> std::io::Result<Vec<crate::codex_state::CodexThreadRef>> {
        if archived_threads_view {
            crate::codex_state::archived_threads_for_cwd(Path::new(&folder.path))
        } else {
            crate::codex_state::threads_for_cwd(Path::new(&folder.path))
        }
    }
}
mod snapshot {
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
}

pub(super) use entry::build_codex_history_entry;
pub(super) use merge::merge_codex_threads;
