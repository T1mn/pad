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
mod history_claude {
    use super::activity::merge_or_insert_thread;
    use crate::claude_history::ClaudeThreadRef;
    use crate::model::{AgentState, AgentType};
    use std::collections::HashMap;
    use std::path::Path;

    use super::super::display::{best_thread_title, clean_title};
    use super::super::model::{SidebarFolder, SidebarThread, ThreadActivityOverride};

    pub(super) fn merge_claude_threads(
        folder: &mut SidebarFolder,
        activity_overrides: &[ThreadActivityOverride],
        thread_sort_activity: &HashMap<String, i64>,
        claude_threads: Option<&[ClaudeThreadRef]>,
        archived_threads_view: bool,
    ) -> usize {
        let Some(threads) = claude_threads else {
            return 0;
        };

        let mut merged = 0usize;
        for thread in threads
            .iter()
            .filter(|thread| thread_matches_folder(thread, &folder.path))
        {
            let sort_updated_at = initial_sort_updated_at(thread.updated_at, archived_threads_view);
            let history_entry = SidebarThread {
                key: format!("claude:{}", thread.session_id),
                folder_key: folder.key.clone(),
                working_dir: folder.path.clone(),
                folder_label: folder.label.clone(),
                agent_type: AgentType::Claude,
                session_id: Some(thread.session_id.clone()),
                transcript_path: Some(thread.transcript_path.to_string_lossy().to_string()),
                session_provider_name: None,
                title: best_thread_title(thread.title.as_deref(), Some(thread.session_id.as_str())),
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

    fn thread_matches_folder(thread: &ClaudeThreadRef, folder_path: &str) -> bool {
        thread.cwd == Path::new(folder_path) || thread.cwd.to_string_lossy() == folder_path
    }

    fn initial_sort_updated_at(updated_at: i64, archived_threads_view: bool) -> i64 {
        if archived_threads_view {
            updated_at
        } else {
            0
        }
    }
}
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
mod history_opencode {
    use super::activity::merge_or_insert_thread;
    use crate::model::{AgentState, AgentType};
    use crate::opencode_history::OpenCodeThreadRef;
    use std::collections::HashMap;
    use std::path::Path;

    use super::super::display::{best_thread_title, clean_title};
    use super::super::model::{SidebarFolder, SidebarThread, ThreadActivityOverride};

    pub(super) fn merge_opencode_threads(
        folder: &mut SidebarFolder,
        activity_overrides: &[ThreadActivityOverride],
        thread_sort_activity: &HashMap<String, i64>,
        opencode_threads: Option<&[OpenCodeThreadRef]>,
        archived_threads_view: bool,
    ) -> usize {
        let Some(threads) = opencode_threads else {
            return 0;
        };

        let mut merged = 0usize;
        for thread in threads
            .iter()
            .filter(|thread| thread_matches_folder(thread, &folder.path))
        {
            let sort_updated_at = initial_sort_updated_at(thread.updated_at, archived_threads_view);
            let subtitle = thread
                .last_user_message
                .as_deref()
                .and_then(clean_title)
                .or_else(|| thread.model_name.as_deref().and_then(clean_title));
            let history_entry = SidebarThread {
                key: format!("opencode:{}", thread.session_id),
                folder_key: folder.key.clone(),
                working_dir: folder.path.clone(),
                folder_label: folder.label.clone(),
                agent_type: AgentType::OpenCode,
                session_id: Some(thread.session_id.clone()),
                transcript_path: Some(thread.db_path.to_string_lossy().to_string()),
                session_provider_name: thread.provider_name.clone(),
                title: best_thread_title(thread.title.as_deref(), Some(thread.session_id.as_str())),
                upstream_title: thread.title.as_deref().and_then(clean_title),
                generated_title: None,
                subtitle,
                title_override: None,
                note: None,
                share_url: thread.share_url.clone(),
                cost: thread.cost.clone(),
                token_summary: thread.token_summary.clone(),
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
                last_user_prompt: thread.last_user_message.clone(),
                last_assistant_message: thread.last_assistant_message.clone(),
                has_unread_stop: false,
                archived: thread.archived,
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

    fn thread_matches_folder(thread: &OpenCodeThreadRef, folder_path: &str) -> bool {
        thread.cwd == Path::new(folder_path) || thread.cwd.to_string_lossy() == folder_path
    }

    fn initial_sort_updated_at(updated_at: i64, archived_threads_view: bool) -> i64 {
        if archived_threads_view {
            updated_at
        } else {
            0
        }
    }
}
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
mod meta {
    use crate::model::AgentType;
    use crate::thread_meta::{ThreadMeta, ThreadMetaKey};
    use std::collections::{HashMap, HashSet};
    use std::io;
    use std::sync::Arc;

    use super::super::display::clean_title;
    use super::super::model::{SidebarFolder, SidebarThread};

    pub(super) fn apply_thread_metadata(folders: &mut HashMap<String, SidebarFolder>) {
        let keys = collect_thread_meta_keys(folders);
        if keys.is_empty() {
            return;
        }

        let Ok(meta_map) = crate::thread_meta::load_thread_meta_batch(&keys) else {
            crate::log_debug!(
                "thread_meta: failed to load batch metadata for {} threads",
                keys.len()
            );
            return;
        };

        for folder in folders.values_mut() {
            for thread in &mut folder.threads {
                apply_thread_meta_lookup(Arc::make_mut(thread), &meta_map);
            }
        }
    }

    fn collect_thread_meta_keys(folders: &HashMap<String, SidebarFolder>) -> Vec<ThreadMetaKey> {
        let mut keys = Vec::new();
        let mut seen = HashSet::new();

        for folder in folders.values() {
            for thread in &folder.threads {
                let Some(session_id) = thread.session_id.as_deref() else {
                    continue;
                };
                let key = ThreadMetaKey::new(thread.agent_type.to_string(), session_id);
                if seen.insert(key.clone()) {
                    keys.push(key);
                }
            }
        }

        keys
    }

    fn apply_thread_meta_lookup(
        thread: &mut SidebarThread,
        meta_map: &HashMap<ThreadMetaKey, ThreadMeta>,
    ) {
        let Some(session_id) = thread.session_id.as_deref() else {
            return;
        };
        let key = ThreadMetaKey::new(thread.agent_type.to_string(), session_id);
        if let Some(meta) = meta_map.get(&key) {
            apply_thread_meta(thread, meta);
        }
    }

    pub(super) fn apply_thread_meta(thread: &mut SidebarThread, meta: &ThreadMeta) {
        thread.title_override = meta.title_override.clone();
        thread.generated_title = meta.generated_title.clone();
        thread.note = meta.note.clone();
        thread.pinned = meta.pinned;
        thread.tags = meta.tags.clone();
        thread.deleted = meta.deleted;

        if let Some(override_title) = meta.title_override.as_deref().and_then(clean_title) {
            thread.title = override_title;
        }
    }

    pub(super) fn load_thread_meta_for_panel(
        agent_type: &AgentType,
        session_id: &str,
    ) -> io::Result<Option<ThreadMeta>> {
        crate::thread_meta::load_thread_meta(&agent_type.to_string(), session_id)
    }
}
mod seed {
    use crate::claude_history::ClaudeThreadRef;
    use crate::gemini_history::GeminiThreadRef;
    use crate::grok_history::GrokThreadRef;
    use crate::model::AgentPanel;
    use std::collections::HashMap;

    use super::super::display::folder_display_label;
    use super::super::model::{SidebarFolder, ThreadActivityOverride};

    pub(super) fn seed_live_folders(
        folders: &mut HashMap<String, SidebarFolder>,
        panels: &[AgentPanel],
    ) {
        for panel in panels {
            ensure_folder(folders, &panel.working_dir);
        }
    }

    pub(super) fn seed_history_folders(
        folders: &mut HashMap<String, SidebarFolder>,
        archived_threads_view: bool,
        claude_threads: Option<&[ClaudeThreadRef]>,
        gemini_threads: Option<&[GeminiThreadRef]>,
        grok_threads: Option<&[GrokThreadRef]>,
        opencode_threads: Option<&[crate::opencode_history::OpenCodeThreadRef]>,
    ) {
        let codex_threads = if archived_threads_view {
            crate::codex_state::all_archived_threads()
        } else {
            crate::codex_state::all_threads()
        };

        if let Ok(codex_threads) = codex_threads {
            for thread in codex_threads {
                if super::super::search::is_subagent_source(thread.source.as_deref()) {
                    continue;
                }
                ensure_folder(folders, &thread.cwd.to_string_lossy());
            }
        }

        for thread in claude_threads.unwrap_or(&[]) {
            ensure_folder(folders, &thread.cwd.to_string_lossy());
        }

        for thread in gemini_threads.unwrap_or(&[]) {
            if thread.kind == "subagent" {
                continue;
            }
            ensure_folder(folders, &thread.cwd.to_string_lossy());
        }

        for thread in grok_threads.unwrap_or(&[]) {
            ensure_folder(folders, &thread.cwd.to_string_lossy());
        }

        for thread in opencode_threads.unwrap_or(&[]) {
            ensure_folder(folders, &thread.cwd.to_string_lossy());
        }
    }

    pub(super) fn seed_activity_folders(
        folders: &mut HashMap<String, SidebarFolder>,
        activity_overrides: &[ThreadActivityOverride],
    ) {
        for activity in activity_overrides {
            ensure_folder(folders, &activity.working_dir);
        }
    }

    fn ensure_folder(folders: &mut HashMap<String, SidebarFolder>, folder_key: &str) {
        folders
            .entry(folder_key.to_string())
            .or_insert_with(|| SidebarFolder {
                key: folder_key.to_string(),
                path: folder_key.to_string(),
                label: folder_display_label(folder_key),
                updated_at: 0,
                threads: Vec::new(),
            });
    }
}
mod sources {
    use crate::claude_history::ClaudeThreadRef;
    use crate::gemini_history::GeminiThreadRef;
    use crate::grok_history::GrokThreadRef;
    use crate::model::AgentType;
    use crate::opencode_history::OpenCodeThreadRef;
    use crate::session_cache::SessionCacheSnapshot;
    use std::collections::HashMap;

    pub(super) struct HistorySources {
        pub(super) codex_session_snapshots: HashMap<String, SessionCacheSnapshot>,
        pub(super) claude_threads: Option<Vec<ClaudeThreadRef>>,
        pub(super) gemini_threads: Option<Vec<GeminiThreadRef>>,
        pub(super) grok_threads: Option<Vec<GrokThreadRef>>,
        pub(super) opencode_threads: Option<Vec<OpenCodeThreadRef>>,
    }

    pub(super) fn load_history_sources(
        live_only: bool,
        archived_threads_view: bool,
    ) -> HistorySources {
        HistorySources {
            codex_session_snapshots: load_codex_session_snapshots(live_only, archived_threads_view),
            claude_threads: load_claude_threads(live_only, archived_threads_view),
            gemini_threads: load_gemini_threads(live_only, archived_threads_view),
            grok_threads: load_grok_threads(live_only, archived_threads_view),
            opencode_threads: load_opencode_threads(live_only, archived_threads_view),
        }
    }

    fn load_grok_threads(
        live_only: bool,
        archived_threads_view: bool,
    ) -> Option<Vec<GrokThreadRef>> {
        if live_only || archived_threads_view {
            None
        } else {
            crate::grok_history::all_threads().ok()
        }
    }

    fn load_codex_session_snapshots(
        live_only: bool,
        archived_threads_view: bool,
    ) -> HashMap<String, SessionCacheSnapshot> {
        if !live_only || archived_threads_view {
            crate::session_cache::load_snapshots_by_agent_type(&AgentType::Codex)
        } else {
            HashMap::new()
        }
    }

    fn load_claude_threads(
        live_only: bool,
        archived_threads_view: bool,
    ) -> Option<Vec<ClaudeThreadRef>> {
        if archived_threads_view {
            crate::claude_history::all_archived_threads().ok()
        } else if live_only {
            None
        } else {
            crate::claude_history::all_threads().ok()
        }
    }

    fn load_gemini_threads(
        live_only: bool,
        archived_threads_view: bool,
    ) -> Option<Vec<GeminiThreadRef>> {
        if archived_threads_view {
            crate::gemini_history::all_archived_threads().ok()
        } else if live_only {
            None
        } else {
            crate::gemini_history::all_threads().ok()
        }
    }

    fn load_opencode_threads(
        live_only: bool,
        archived_threads_view: bool,
    ) -> Option<Vec<OpenCodeThreadRef>> {
        if archived_threads_view {
            crate::opencode_history::all_archived_threads().ok()
        } else if live_only {
            None
        } else {
            crate::opencode_history::all_threads().ok()
        }
    }
}
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
pub(crate) mod tests;
