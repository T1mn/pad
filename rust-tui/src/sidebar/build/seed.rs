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
