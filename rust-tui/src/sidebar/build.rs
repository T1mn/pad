mod activity;
mod finalize {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::super::model::SidebarFolder;
    use super::super::sort::thread_sort_key;
    use super::activity::apply_sort_activity;

    pub(super) fn finalize_folder_threads(
        folder: &mut SidebarFolder,
        thread_sort_activity: &HashMap<String, i64>,
        retain_deleted_with_live_pane_only: bool,
    ) {
        if retain_deleted_with_live_pane_only {
            folder
                .threads
                .retain(|thread| !thread.deleted || thread.live_pane_id.is_some());
        }
        for thread in &mut folder.threads {
            apply_sort_activity(Arc::make_mut(thread), thread_sort_activity);
        }
        folder.threads.sort_by(thread_sort_key);
        folder.updated_at = folder
            .threads
            .first()
            .map(|thread| thread.sort_timestamp())
            .unwrap_or_default();
    }
}
mod folder;
mod history_claude;
mod history_codex;
mod history_gemini;
mod history_grok {
    use super::activity::merge_or_insert_thread;
    use crate::grok_history::GrokThreadRef;
    use crate::model::{AgentState, AgentType};
    use std::collections::HashMap;
    use std::path::Path;

    use super::super::display::{best_thread_title, clean_title};
    use super::super::model::{SidebarFolder, SidebarThread, ThreadActivityOverride};

    pub(super) fn merge_grok_threads(
        folder: &mut SidebarFolder,
        activity_overrides: &[ThreadActivityOverride],
        thread_sort_activity: &HashMap<String, i64>,
        grok_threads: Option<&[GrokThreadRef]>,
        archived_threads_view: bool,
    ) -> usize {
        if archived_threads_view {
            return 0;
        }
        let Some(threads) = grok_threads else {
            return 0;
        };
        let mut merged = 0;
        for thread in threads
            .iter()
            .filter(|thread| thread.cwd == Path::new(&folder.path))
        {
            let history_entry = SidebarThread {
                key: format!("grok:{}", thread.session_id),
                folder_key: folder.key.clone(),
                working_dir: folder.path.clone(),
                folder_label: folder.label.clone(),
                agent_type: AgentType::Grok,
                session_id: Some(thread.session_id.clone()),
                transcript_path: Some(thread.transcript_path.to_string_lossy().to_string()),
                session_provider_name: None,
                title: best_thread_title(thread.title.as_deref(), Some(&thread.session_id)),
                upstream_title: thread.title.as_deref().and_then(clean_title),
                generated_title: None,
                subtitle: thread.model_name.as_deref().and_then(clean_title),
                title_override: None,
                note: None,
                share_url: None,
                cost: None,
                token_summary: None,
                tags: Vec::new(),
                pinned: false,
                updated_at: thread.updated_at,
                sort_updated_at: 0,
                live_pane_id: None,
                live_location: None,
                state: AgentState::Idle,
                is_active: false,
                cached_preview_turns: Default::default(),
                session_cache_state: None,
                last_user_prompt: None,
                last_assistant_message: None,
                has_unread_stop: false,
                archived: false,
                deleted: false,
            };
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
}
mod history_opencode;
mod live;
mod logging {
    use std::time::{Duration, Instant};

    pub(super) fn log_sidebar_stage(
        label: &str,
        started_at: Instant,
        folder_count: usize,
        item_count: usize,
    ) {
        let elapsed = started_at.elapsed();
        if elapsed >= Duration::from_millis(8) {
            crate::log_debug!(
                "sidebar.build: stage={} elapsed_ms={} folders={} items={}",
                label,
                elapsed.as_millis(),
                folder_count,
                item_count
            );
        }
    }

    pub(super) fn log_slow_folder(path: &str, thread_count: usize, started_at: Instant) {
        if started_at.elapsed() >= Duration::from_millis(20) {
            crate::log_debug!(
                "sidebar.build: folder_slow path={} threads={} elapsed_ms={}",
                path,
                thread_count,
                started_at.elapsed().as_millis()
            );
        }
    }

    pub(super) struct BuildLogStats {
        pub(super) live_panel_threads: usize,
        pub(super) hidden_live_panels: usize,
        pub(super) codex_history_threads: usize,
        pub(super) claude_history_threads: usize,
        pub(super) gemini_history_threads: usize,
        pub(super) grok_history_threads: usize,
        pub(super) opencode_history_threads: usize,
    }

    impl BuildLogStats {
        pub(super) fn new() -> Self {
            Self {
                live_panel_threads: 0,
                hidden_live_panels: 0,
                codex_history_threads: 0,
                claude_history_threads: 0,
                gemini_history_threads: 0,
                grok_history_threads: 0,
                opencode_history_threads: 0,
            }
        }
    }

    pub(super) fn log_total_build(started_at: Instant, folder_count: usize, stats: &BuildLogStats) {
        if started_at.elapsed() >= Duration::from_millis(20) {
            crate::log_debug!(
                "sidebar.build: total elapsed_ms={} folders={} live_threads={} hidden_live_panels={} codex_history_threads={} claude_history_threads={} gemini_history_threads={} grok_history_threads={} opencode_history_threads={}",
                started_at.elapsed().as_millis(),
                folder_count,
                stats.live_panel_threads,
                stats.hidden_live_panels,
                stats.codex_history_threads,
                stats.claude_history_threads,
                stats.gemini_history_threads,
                stats.grok_history_threads,
                stats.opencode_history_threads
            );
        }
    }
}
mod meta;
mod seed;
mod sources;
mod trash;

use crate::app::state::ThreadListView;
use crate::model::AgentPanel;
use std::collections::HashMap;
use std::time::Instant;

use super::model::{SidebarFolder, ThreadActivityOverride};
use super::sort::folder_sort_key;
use finalize::finalize_folder_threads;
use folder::{populate_folder_threads, FolderBuildContext};
use live::build_live_panel_fallback_folders;
use logging::{log_sidebar_stage, log_total_build, BuildLogStats};
use meta::apply_thread_metadata;
use seed::{seed_activity_folders, seed_history_folders, seed_live_folders};
use sources::load_history_sources;

pub fn build_sidebar_folders(
    panels: &[AgentPanel],
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    thread_list_view: ThreadListView,
    live_only: bool,
) -> Vec<SidebarFolder> {
    if thread_list_view == ThreadListView::Trash {
        return trash::build_trash_folders();
    }

    let build_started_at = Instant::now();
    let mut stats = BuildLogStats::new();
    let mut folders: HashMap<String, SidebarFolder> = HashMap::new();
    let archived_threads_view = thread_list_view == ThreadListView::Archived;
    let history_sources = load_history_sources(live_only, archived_threads_view);

    let seed_live_started_at = Instant::now();
    if !archived_threads_view {
        seed_live_folders(&mut folders, panels);
    }
    log_sidebar_stage("seed_live_folders", seed_live_started_at, folders.len(), 0);

    if !live_only || archived_threads_view {
        let seed_history_started_at = Instant::now();
        seed_history_folders(
            &mut folders,
            archived_threads_view,
            history_sources.claude_threads.as_deref(),
            history_sources.gemini_threads.as_deref(),
            history_sources.grok_threads.as_deref(),
            history_sources.opencode_threads.as_deref(),
        );
        log_sidebar_stage(
            "seed_history_folders",
            seed_history_started_at,
            folders.len(),
            0,
        );
    }
    if !archived_threads_view {
        let seed_activity_started_at = Instant::now();
        seed_activity_folders(&mut folders, activity_overrides);
        log_sidebar_stage(
            "seed_activity_folders",
            seed_activity_started_at,
            folders.len(),
            activity_overrides.len(),
        );
    }

    let folder_context = FolderBuildContext {
        panels,
        activity_overrides,
        thread_sort_activity,
        history_sources: &history_sources,
        live_only,
        archived_threads_view,
    };
    for folder in folders.values_mut() {
        populate_folder_threads(folder, &folder_context, &mut stats);
    }

    apply_thread_metadata(&mut folders);
    for folder in folders.values_mut() {
        finalize_folder_threads(folder, thread_sort_activity, true);
    }

    let final_sort_started_at = Instant::now();
    let mut values = folders
        .into_values()
        .filter(|folder| !folder.threads.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() && live_only && !archived_threads_view && !panels.is_empty() {
        values = build_live_panel_fallback_folders(panels);
        crate::log_debug!(
            "sidebar.build: live_fallback folders={} panels={}",
            values.len(),
            panels.len()
        );
    }
    values.sort_by(folder_sort_key);
    log_sidebar_stage("final_sort", final_sort_started_at, values.len(), 0);
    log_total_build(build_started_at, values.len(), &stats);
    values
}

pub use live::thread_from_live_panel;

#[cfg(test)]
mod tests;
