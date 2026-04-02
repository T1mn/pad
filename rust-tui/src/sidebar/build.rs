mod activity;
mod history_claude;
mod history_codex;
mod history_gemini;
mod live;
mod meta;

use crate::claude_history::ClaudeThreadRef;
use crate::gemini_history::GeminiThreadRef;
use crate::model::{AgentPanel, AgentType};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::display::folder_display_label;
use super::model::{SidebarFolder, ThreadActivityOverride};
use super::sort::{folder_sort_key, thread_sort_key};
use activity::apply_sort_activity;
use history_claude::merge_claude_threads;
use history_codex::merge_codex_threads;
use history_gemini::merge_gemini_threads;
use live::{build_live_panel_fallback_folders, should_hide_live_panel};
use meta::apply_thread_metadata;

pub fn build_sidebar_folders(
    panels: &[AgentPanel],
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    startup_thread_sort_activity: &HashMap<String, i64>,
    archived_threads_view: bool,
    live_only: bool,
) -> Vec<SidebarFolder> {
    let build_started_at = Instant::now();
    let mut folders: HashMap<String, SidebarFolder> = HashMap::new();
    let mut live_panel_threads = 0usize;
    let mut hidden_live_panels = 0usize;
    let mut codex_history_threads = 0usize;
    let mut claude_history_threads = 0usize;
    let mut gemini_history_threads = 0usize;
    let codex_session_snapshots = if !live_only || archived_threads_view {
        crate::session_cache::load_snapshots_by_agent_type(&AgentType::Codex)
    } else {
        HashMap::new()
    };
    let claude_threads = if archived_threads_view {
        crate::claude_history::all_archived_threads().ok()
    } else if live_only {
        None
    } else {
        crate::claude_history::all_threads().ok()
    };
    let gemini_threads = if archived_threads_view {
        crate::gemini_history::all_archived_threads().ok()
    } else if live_only {
        None
    } else {
        crate::gemini_history::all_threads().ok()
    };
    let seed_live_started_at = Instant::now();
    if !archived_threads_view {
        for panel in panels {
            let folder_key = panel.working_dir.clone();
            let folder_label = folder_display_label(&panel.working_dir);
            folders
                .entry(folder_key.clone())
                .or_insert_with(|| SidebarFolder {
                    key: folder_key.clone(),
                    path: panel.working_dir.clone(),
                    label: folder_label.clone(),
                    updated_at: 0,
                    threads: Vec::new(),
                });
        }
    }
    log_sidebar_stage("seed_live_folders", seed_live_started_at, folders.len(), 0);

    if !live_only || archived_threads_view {
        let seed_history_started_at = Instant::now();
        seed_history_folders(
            &mut folders,
            archived_threads_view,
            claude_threads.as_deref(),
            gemini_threads.as_deref(),
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

    let folder_paths = folders.keys().cloned().collect::<Vec<_>>();
    for folder_path in &folder_paths {
        if let Some(folder) = folders.get_mut(folder_path) {
            let folder_started_at = Instant::now();
            if !archived_threads_view {
                let live_panels = panels
                    .iter()
                    .filter(|panel| panel.working_dir == *folder_path)
                    .collect::<Vec<_>>();
                for panel in live_panels {
                    if should_hide_live_panel(panel) {
                        hidden_live_panels += 1;
                        continue;
                    }
                    folder.threads.push(live::thread_from_live_panel(panel));
                    live_panel_threads += 1;
                }
            }

            if !live_only || archived_threads_view {
                codex_history_threads += merge_codex_threads(
                    folder,
                    activity_overrides,
                    thread_sort_activity,
                    startup_thread_sort_activity,
                    &codex_session_snapshots,
                    archived_threads_view,
                );
                claude_history_threads += merge_claude_threads(
                    folder,
                    activity_overrides,
                    thread_sort_activity,
                    startup_thread_sort_activity,
                    claude_threads.as_deref(),
                    archived_threads_view,
                );
                gemini_history_threads += merge_gemini_threads(
                    folder,
                    activity_overrides,
                    thread_sort_activity,
                    startup_thread_sort_activity,
                    gemini_threads.as_deref(),
                    archived_threads_view,
                );
            }
            for thread in &mut folder.threads {
                apply_sort_activity(thread, thread_sort_activity, startup_thread_sort_activity);
            }
            folder.threads.sort_by(thread_sort_key);
            folder.updated_at = folder
                .threads
                .first()
                .map(|thread| thread.sort_timestamp())
                .unwrap_or_default();
            if folder_started_at.elapsed() >= Duration::from_millis(20) {
                crate::log_debug!(
                    "sidebar.build: folder_slow path={} threads={} elapsed_ms={}",
                    folder.path,
                    folder.threads.len(),
                    folder_started_at.elapsed().as_millis()
                );
            }
        }
    }

    apply_thread_metadata(&mut folders);
    for folder in folders.values_mut() {
        for thread in &mut folder.threads {
            apply_sort_activity(thread, thread_sort_activity, startup_thread_sort_activity);
        }
        folder.threads.sort_by(thread_sort_key);
        folder.updated_at = folder
            .threads
            .first()
            .map(|thread| thread.sort_timestamp())
            .unwrap_or_default();
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
    if build_started_at.elapsed() >= Duration::from_millis(20) {
        crate::log_debug!(
            "sidebar.build: total elapsed_ms={} folders={} live_threads={} hidden_live_panels={} codex_history_threads={} claude_history_threads={} gemini_history_threads={}",
            build_started_at.elapsed().as_millis(),
            values.len(),
            live_panel_threads,
            hidden_live_panels,
            codex_history_threads,
            claude_history_threads,
            gemini_history_threads
        );
    }
    values
}

fn log_sidebar_stage(label: &str, started_at: Instant, folder_count: usize, item_count: usize) {
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

fn seed_history_folders(
    folders: &mut HashMap<String, SidebarFolder>,
    archived_threads_view: bool,
    claude_threads: Option<&[ClaudeThreadRef]>,
    gemini_threads: Option<&[GeminiThreadRef]>,
) {
    let codex_threads = if archived_threads_view {
        crate::codex_state::all_archived_threads()
    } else {
        crate::codex_state::all_threads()
    };

    if let Ok(codex_threads) = codex_threads {
        for thread in codex_threads {
            if super::search::is_subagent_source(thread.source.as_deref()) {
                continue;
            }
            let folder_key = thread.cwd.to_string_lossy().to_string();
            folders
                .entry(folder_key.clone())
                .or_insert_with(|| SidebarFolder {
                    key: folder_key.clone(),
                    path: folder_key.clone(),
                    label: folder_display_label(&folder_key),
                    updated_at: 0,
                    threads: Vec::new(),
                });
        }
    }

    for thread in claude_threads.unwrap_or(&[]) {
        let folder_key = thread.cwd.to_string_lossy().to_string();
        folders
            .entry(folder_key.clone())
            .or_insert_with(|| SidebarFolder {
                key: folder_key.clone(),
                path: folder_key.clone(),
                label: folder_display_label(&folder_key),
                updated_at: 0,
                threads: Vec::new(),
            });
    }

    for thread in gemini_threads.unwrap_or(&[]) {
        if thread.kind == "subagent" {
            continue;
        }
        let folder_key = thread.cwd.to_string_lossy().to_string();
        folders
            .entry(folder_key.clone())
            .or_insert_with(|| SidebarFolder {
                key: folder_key.clone(),
                path: folder_key.clone(),
                label: folder_display_label(&folder_key),
                updated_at: 0,
                threads: Vec::new(),
            });
    }
}

fn seed_activity_folders(
    folders: &mut HashMap<String, SidebarFolder>,
    activity_overrides: &[ThreadActivityOverride],
) {
    for activity in activity_overrides {
        let folder_key = activity.working_dir.clone();
        folders
            .entry(folder_key.clone())
            .or_insert_with(|| SidebarFolder {
                key: folder_key.clone(),
                path: folder_key.clone(),
                label: folder_display_label(&folder_key),
                updated_at: 0,
                threads: Vec::new(),
            });
    }
}

pub use live::thread_from_live_panel;

#[cfg(test)]
mod tests;
