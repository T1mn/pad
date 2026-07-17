use super::finalize::finalize_folder_threads;
use super::history_claude::merge_claude_threads;
use super::history_codex::merge_codex_threads;
use super::history_gemini::merge_gemini_threads;
use super::history_grok::merge_grok_threads;
use super::history_opencode::merge_opencode_threads;
use super::live::{self, should_hide_live_panel};
use super::logging::{log_slow_folder, BuildLogStats};
use super::sources::HistorySources;
use crate::model::AgentPanel;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::super::model::{SidebarFolder, ThreadActivityOverride};

pub(super) struct FolderBuildContext<'a> {
    pub(super) panels: &'a [AgentPanel],
    pub(super) activity_overrides: &'a [ThreadActivityOverride],
    pub(super) thread_sort_activity: &'a HashMap<String, i64>,
    pub(super) startup_thread_sort_activity: &'a HashMap<String, i64>,
    pub(super) history_sources: &'a HistorySources,
    pub(super) live_only: bool,
    pub(super) archived_threads_view: bool,
}

pub(super) fn populate_folder_threads(
    folder: &mut SidebarFolder,
    ctx: &FolderBuildContext<'_>,
    stats: &mut BuildLogStats,
) {
    let folder_started_at = Instant::now();
    if !ctx.archived_threads_view {
        merge_live_panel_threads(folder, ctx, stats);
    }

    if !ctx.live_only || ctx.archived_threads_view {
        merge_history_threads(folder, ctx, stats);
    }

    finalize_folder_threads(
        folder,
        ctx.thread_sort_activity,
        ctx.startup_thread_sort_activity,
        false,
    );
    log_slow_folder(&folder.path, folder.threads.len(), folder_started_at);
}

fn merge_live_panel_threads(
    folder: &mut SidebarFolder,
    ctx: &FolderBuildContext<'_>,
    stats: &mut BuildLogStats,
) {
    for panel in ctx.panels {
        if panel.working_dir != folder.path {
            continue;
        }
        if should_hide_live_panel(panel) {
            stats.hidden_live_panels += 1;
            continue;
        }
        folder
            .threads
            .push(Arc::new(live::thread_from_live_panel(panel)));
        stats.live_panel_threads += 1;
    }
}

fn merge_history_threads(
    folder: &mut SidebarFolder,
    ctx: &FolderBuildContext<'_>,
    stats: &mut BuildLogStats,
) {
    stats.codex_history_threads += merge_codex_threads(
        folder,
        ctx.activity_overrides,
        ctx.thread_sort_activity,
        ctx.startup_thread_sort_activity,
        &ctx.history_sources.codex_session_snapshots,
        ctx.archived_threads_view,
    );
    stats.claude_history_threads += merge_claude_threads(
        folder,
        ctx.activity_overrides,
        ctx.thread_sort_activity,
        ctx.startup_thread_sort_activity,
        ctx.history_sources.claude_threads.as_deref(),
        ctx.archived_threads_view,
    );
    stats.gemini_history_threads += merge_gemini_threads(
        folder,
        ctx.activity_overrides,
        ctx.thread_sort_activity,
        ctx.startup_thread_sort_activity,
        ctx.history_sources.gemini_threads.as_deref(),
        ctx.archived_threads_view,
    );
    stats.grok_history_threads += merge_grok_threads(
        folder,
        ctx.activity_overrides,
        ctx.thread_sort_activity,
        ctx.startup_thread_sort_activity,
        ctx.history_sources.grok_threads.as_deref(),
        ctx.archived_threads_view,
    );
    stats.opencode_history_threads += merge_opencode_threads(
        folder,
        ctx.activity_overrides,
        ctx.thread_sort_activity,
        ctx.startup_thread_sort_activity,
        ctx.history_sources.opencode_threads.as_deref(),
        ctx.archived_threads_view,
    );
}
