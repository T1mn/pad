mod activity;
mod finalize;
mod folder;
mod history_claude;
mod history_codex;
mod history_gemini;
mod history_opencode;
mod live;
mod logging;
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
    startup_thread_sort_activity: &HashMap<String, i64>,
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
        startup_thread_sort_activity,
        history_sources: &history_sources,
        live_only,
        archived_threads_view,
    };
    let folder_paths = folders.keys().cloned().collect::<Vec<_>>();
    for folder_path in &folder_paths {
        if let Some(folder) = folders.get_mut(folder_path) {
            populate_folder_threads(folder, &folder_context, &mut stats);
        }
    }

    apply_thread_metadata(&mut folders);
    for folder in folders.values_mut() {
        finalize_folder_threads(
            folder,
            thread_sort_activity,
            startup_thread_sort_activity,
            true,
        );
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
