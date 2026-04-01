use crate::claude_history::ClaudeThreadRef;
use crate::codex_state::CodexThreadRef;
use crate::gemini_history::GeminiThreadRef;
use crate::model::{AgentPanel, AgentState, AgentType};
use crate::session_cache::SessionCacheSnapshot;
use crate::thread_meta::{ThreadMeta, ThreadMetaKey};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use super::display::{best_thread_title, clean_title, folder_display_label};
use super::model::{SidebarFolder, SidebarThread, ThreadActivityOverride};
use super::search::is_subagent_source;
use super::sort::{folder_sort_key, thread_sort_key};

pub fn build_sidebar_folders(
    panels: &[AgentPanel],
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
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
                    folder.threads.push(thread_from_live_panel(panel));
                    live_panel_threads += 1;
                }
            }

            if !live_only || archived_threads_view {
                codex_history_threads += merge_codex_threads(
                    folder,
                    activity_overrides,
                    thread_sort_activity,
                    &codex_session_snapshots,
                    archived_threads_view,
                );
                claude_history_threads += merge_claude_threads(
                    folder,
                    activity_overrides,
                    thread_sort_activity,
                    claude_threads.as_deref(),
                );
                gemini_history_threads += merge_gemini_threads(
                    folder,
                    activity_overrides,
                    thread_sort_activity,
                    gemini_threads.as_deref(),
                );
            }
            for thread in &mut folder.threads {
                apply_sort_activity(thread, thread_sort_activity);
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
            apply_sort_activity(thread, thread_sort_activity);
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

fn apply_thread_metadata(folders: &mut HashMap<String, SidebarFolder>) {
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
            apply_thread_meta_lookup(thread, &meta_map);
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

fn apply_thread_meta(thread: &mut SidebarThread, meta: &ThreadMeta) {
    thread.title_override = meta.title_override.clone();
    thread.note = meta.note.clone();
    thread.pinned = meta.pinned;
    thread.tags = meta.tags.clone();

    if let Some(override_title) = meta.title_override.as_deref().and_then(clean_title) {
        thread.title = override_title;
    }
}

fn load_thread_meta_for_panel(
    agent_type: &AgentType,
    session_id: &str,
) -> io::Result<Option<ThreadMeta>> {
    crate::thread_meta::load_thread_meta(&agent_type.to_string(), session_id)
}

fn build_live_panel_fallback_folders(panels: &[AgentPanel]) -> Vec<SidebarFolder> {
    let mut folders: HashMap<String, SidebarFolder> = HashMap::new();

    for panel in panels {
        let folder_key = panel.working_dir.clone();
        let folder = folders
            .entry(folder_key.clone())
            .or_insert_with(|| SidebarFolder {
                key: folder_key.clone(),
                path: panel.working_dir.clone(),
                label: folder_display_label(&panel.working_dir),
                updated_at: 0,
                threads: Vec::new(),
            });
        folder.threads.push(thread_from_live_panel(panel));
    }

    apply_thread_metadata(&mut folders);

    let mut values = folders
        .into_values()
        .filter(|folder| !folder.threads.is_empty())
        .collect::<Vec<_>>();
    for folder in &mut values {
        folder.threads.sort_by(thread_sort_key);
        folder.updated_at = folder
            .threads
            .first()
            .map(|thread| thread.sort_timestamp())
            .unwrap_or_default();
    }
    values.sort_by(folder_sort_key);
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
            if is_subagent_source(thread.source.as_deref()) {
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

fn merge_codex_threads(
    folder: &mut SidebarFolder,
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    codex_session_snapshots: &HashMap<String, SessionCacheSnapshot>,
    archived_threads_view: bool,
) -> usize {
    let threads = if archived_threads_view {
        crate::codex_state::archived_threads_for_cwd(Path::new(&folder.path))
    } else {
        crate::codex_state::threads_for_cwd(Path::new(&folder.path))
    };
    let Ok(threads) = threads else {
        return 0;
    };

    let mut merged = 0usize;
    for thread in threads {
        if is_subagent_source(thread.source.as_deref()) {
            continue;
        }
        let history_entry = build_codex_history_entry(
            folder,
            &thread,
            codex_session_snapshots.get(&thread.thread_id),
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

fn build_codex_history_entry(
    folder: &SidebarFolder,
    thread: &CodexThreadRef,
    snapshot: Option<&SessionCacheSnapshot>,
) -> SidebarThread {
    let title = best_thread_title(thread.title.as_deref(), Some(thread.thread_id.as_str()));
    let mut history_entry = SidebarThread {
        key: format!("codex:{}", thread.thread_id),
        folder_key: folder.key.clone(),
        working_dir: folder.path.clone(),
        folder_label: folder.label.clone(),
        agent_type: AgentType::Codex,
        runtime_source: None,
        session_id: Some(thread.thread_id.clone()),
        transcript_path: Some(thread.rollout_path.to_string_lossy().to_string()),
        title,
        upstream_title: thread.title.as_deref().and_then(clean_title),
        subtitle: None,
        title_override: None,
        note: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: thread.updated_at,
        sort_updated_at: thread.updated_at,
        live_pane_id: None,
        live_location: None,
        pid: None,
        git_info: None,
        state: AgentState::Idle,
        is_active: false,
        cached_preview_turns: Vec::new(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: thread.archived,
    };

    if let Some(snapshot) = snapshot {
        apply_session_cache_snapshot(&mut history_entry, snapshot);
    }

    if history_entry.subtitle.is_none() {
        history_entry.subtitle = thread.first_user_message.as_deref().and_then(clean_title);
    }

    history_entry
}

fn apply_session_cache_snapshot(thread: &mut SidebarThread, snapshot: &SessionCacheSnapshot) {
    if thread.transcript_path.is_none() {
        thread.transcript_path = snapshot.transcript_path.clone();
    }

    if !snapshot.recent_turns.is_empty() {
        thread.cached_preview_turns = snapshot.recent_turns.clone();
    }

    if let Some(prompt) = snapshot
        .last_user_prompt
        .as_deref()
        .and_then(clean_cached_prompt)
        .or_else(|| {
            snapshot
                .recent_turns
                .first()
                .and_then(|turn| clean_cached_prompt(&turn.question))
        })
    {
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

fn clean_cached_prompt(text: &str) -> Option<String> {
    clean_title(&crate::preview_source::codex::normalize_codex_user_text(
        text, None,
    ))
}

fn should_hide_live_panel(panel: &AgentPanel) -> bool {
    let Some(session_id) = panel.agent_session_id.as_deref() else {
        return false;
    };

    match panel.agent_type {
        AgentType::Codex => crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some_and(|thread| thread.archived),
        AgentType::Claude => crate::claude_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some_and(|thread| thread.archived),
        AgentType::Gemini => crate::gemini_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some_and(|thread| thread.archived),
        _ => false,
    }
}

fn merge_claude_threads(
    folder: &mut SidebarFolder,
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    claude_threads: Option<&[ClaudeThreadRef]>,
) -> usize {
    let Some(threads) = claude_threads else {
        return 0;
    };

    let mut merged = 0usize;
    for thread in threads
        .iter()
        .filter(|thread| thread_matches_folder(thread, &folder.path))
    {
        let history_entry = SidebarThread {
            key: format!("claude:{}", thread.session_id),
            folder_key: folder.key.clone(),
            working_dir: folder.path.clone(),
            folder_label: folder.label.clone(),
            agent_type: AgentType::Claude,
            runtime_source: None,
            session_id: Some(thread.session_id.clone()),
            transcript_path: Some(thread.transcript_path.to_string_lossy().to_string()),
            title: best_thread_title(thread.title.as_deref(), Some(thread.session_id.as_str())),
            upstream_title: thread.title.as_deref().and_then(clean_title),
            subtitle: None,
            title_override: None,
            note: None,
            tags: Vec::new(),
            pinned: false,
            updated_at: thread.updated_at,
            sort_updated_at: thread.updated_at,
            live_pane_id: None,
            live_location: None,
            pid: None,
            git_info: None,
            state: AgentState::Idle,
            is_active: false,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
            archived: thread.archived,
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

fn merge_gemini_threads(
    folder: &mut SidebarFolder,
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    gemini_threads: Option<&[GeminiThreadRef]>,
) -> usize {
    let Some(threads) = gemini_threads else {
        return 0;
    };

    let mut merged = 0usize;
    for thread in threads
        .iter()
        .filter(|thread| gemini_thread_matches_folder(thread, &folder.path))
    {
        if thread.kind == "subagent" {
            continue;
        }
        let history_entry = SidebarThread {
            key: format!("gemini:{}", thread.session_id),
            folder_key: folder.key.clone(),
            working_dir: folder.path.clone(),
            folder_label: folder.label.clone(),
            agent_type: AgentType::Gemini,
            runtime_source: None,
            session_id: Some(thread.session_id.clone()),
            transcript_path: Some(thread.transcript_path.to_string_lossy().to_string()),
            title: best_thread_title(thread.title.as_deref(), Some(thread.session_id.as_str())),
            upstream_title: thread.title.as_deref().and_then(clean_title),
            subtitle: thread
                .subtitle
                .as_deref()
                .and_then(clean_title)
                .or_else(|| thread.last_user_message.as_deref().and_then(clean_title)),
            title_override: None,
            note: None,
            tags: Vec::new(),
            pinned: false,
            updated_at: thread.updated_at,
            sort_updated_at: thread.updated_at,
            live_pane_id: None,
            live_location: None,
            pid: None,
            git_info: None,
            state: AgentState::Idle,
            is_active: false,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            last_user_prompt: thread.last_user_message.clone(),
            last_assistant_message: thread.last_assistant_message.clone(),
            has_unread_stop: false,
            archived: thread.archived,
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

fn gemini_thread_matches_folder(thread: &GeminiThreadRef, folder_path: &str) -> bool {
    thread.cwd == Path::new(folder_path) || thread.cwd.to_string_lossy() == folder_path
}

fn merge_or_insert_thread(
    threads: &mut Vec<SidebarThread>,
    mut history_entry: SidebarThread,
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
) {
    apply_activity_override(&mut history_entry, activity_overrides);
    apply_sort_activity(&mut history_entry, thread_sort_activity);
    if let Some(existing) = threads.iter_mut().find(|existing| {
        existing.agent_type == history_entry.agent_type
            && ((existing.session_id.is_some() && existing.session_id == history_entry.session_id)
                || (existing.transcript_path.is_some()
                    && existing.transcript_path == history_entry.transcript_path))
    }) {
        existing.session_id = existing.session_id.clone().or(history_entry.session_id);
        existing.transcript_path = existing
            .transcript_path
            .clone()
            .or(history_entry.transcript_path.clone());
        existing.archived = existing.archived || history_entry.archived;
        if existing.title.trim().is_empty() || existing.title.starts_with('%') {
            existing.title = history_entry.title;
        }
        if existing.upstream_title.is_none() {
            existing.upstream_title = history_entry.upstream_title;
        }
        if existing.subtitle.is_none() {
            existing.subtitle = history_entry
                .last_user_prompt
                .clone()
                .or(history_entry.subtitle.clone());
        }
        if existing.title_override.is_none() {
            existing.title_override = history_entry.title_override;
        }
        if existing.note.is_none() {
            existing.note = history_entry.note;
        }
        if existing.tags.is_empty() {
            existing.tags = history_entry.tags;
        } else {
            for tag in history_entry.tags {
                if !existing
                    .tags
                    .iter()
                    .any(|existing_tag| existing_tag == &tag)
                {
                    existing.tags.push(tag);
                }
            }
        }
        existing.pinned |= history_entry.pinned;
        existing.updated_at = existing.updated_at.max(history_entry.updated_at);
        existing.sort_updated_at = existing.sort_updated_at.max(history_entry.sort_updated_at);
        if existing.runtime_source.is_none() {
            existing.runtime_source = history_entry.runtime_source;
        }
        if existing.cached_preview_turns.is_empty() {
            existing.cached_preview_turns = history_entry.cached_preview_turns.clone();
        }
        if existing.session_cache_state.is_none() {
            existing.session_cache_state = history_entry.session_cache_state;
        }
        if existing.last_user_prompt.is_none() {
            existing.last_user_prompt = history_entry.last_user_prompt.clone();
        }
        if existing.last_assistant_message.is_none() {
            existing.last_assistant_message = history_entry.last_assistant_message.clone();
        }
        apply_activity_override(existing, activity_overrides);
        apply_sort_activity(existing, thread_sort_activity);
        return;
    }

    threads.push(history_entry);
}

fn apply_sort_activity(thread: &mut SidebarThread, thread_sort_activity: &HashMap<String, i64>) {
    if let Some(sort_updated_at) = thread
        .sort_activity_keys()
        .iter()
        .find_map(|key| thread_sort_activity.get(key).copied())
    {
        thread.sort_updated_at = thread.sort_updated_at.max(sort_updated_at);
    }
}

fn apply_activity_override(
    thread: &mut SidebarThread,
    activity_overrides: &[ThreadActivityOverride],
) {
    let Some(override_entry) = activity_overrides.iter().find(|entry| {
        entry.agent_type == thread.agent_type
            && ((entry.session_id.is_some() && entry.session_id == thread.session_id)
                || (entry.transcript_path.is_some()
                    && entry.transcript_path == thread.transcript_path))
    }) else {
        return;
    };

    thread.state = override_entry.state.clone();
    thread.is_active = override_entry.is_active;
    thread.updated_at = thread.updated_at.max(override_entry.updated_at);
    if thread.last_user_prompt.is_none() {
        thread.last_user_prompt = override_entry.last_user_prompt.clone();
    }
    if thread.subtitle.is_none() {
        thread.subtitle = override_entry.last_user_prompt.clone();
    }
    if thread.last_assistant_message.is_none() {
        thread.last_assistant_message = override_entry.last_assistant_message.clone();
    }
    if thread.title.trim().is_empty() || thread.title == "untitled" {
        thread.title = thread
            .upstream_title
            .clone()
            .or_else(|| thread.session_id.clone())
            .unwrap_or_else(|| "untitled".to_string());
    }
}

pub fn thread_from_live_panel(panel: &AgentPanel) -> SidebarThread {
    let updated_at = panel
        .transcript_path
        .as_deref()
        .and_then(|path| file_mtime(Path::new(path)))
        .unwrap_or_default();
    let upstream_title = resolve_live_panel_upstream_title(panel);
    let subtitle = resolve_live_panel_subtitle(panel);
    let fallback_title = panel
        .agent_session_id
        .as_deref()
        .or(Some(panel.pane_id.as_str()));
    let mut thread = SidebarThread {
        key: format!("live:{}", panel.pane_id),
        folder_key: panel.working_dir.clone(),
        working_dir: panel.working_dir.clone(),
        folder_label: folder_display_label(&panel.working_dir),
        agent_type: panel.agent_type.clone(),
        runtime_source: None,
        session_id: panel.agent_session_id.clone(),
        transcript_path: panel.transcript_path.clone(),
        title: upstream_title
            .clone()
            .or_else(|| fallback_title.map(|value| value.to_string()))
            .unwrap_or_else(|| "untitled".to_string()),
        upstream_title,
        subtitle,
        title_override: None,
        note: None,
        tags: Vec::new(),
        pinned: false,
        updated_at,
        sort_updated_at: updated_at,
        live_pane_id: Some(panel.pane_id.clone()),
        live_location: Some(format!(
            "{}:{}.{}",
            panel.session, panel.window_index, panel.pane
        )),
        pid: panel.pid.clone(),
        git_info: panel.git_info.clone(),
        state: panel.state.clone(),
        is_active: panel.is_active,
        cached_preview_turns: panel.cached_preview_turns.clone(),
        session_cache_state: panel.session_cache_state,
        last_user_prompt: panel.last_user_prompt.clone(),
        last_assistant_message: panel.last_assistant_message.clone(),
        has_unread_stop: panel.has_unread_stop,
        archived: false,
    };

    if let Ok(Some(meta)) = panel
        .agent_session_id
        .as_deref()
        .map(|session_id| load_thread_meta_for_panel(&panel.agent_type, session_id))
        .unwrap_or_else(|| Ok(None))
    {
        apply_thread_meta(&mut thread, &meta);
    }

    thread
}

fn resolve_live_panel_upstream_title(panel: &AgentPanel) -> Option<String> {
    let session_id = panel.agent_session_id.as_deref()?;
    match panel.agent_type {
        AgentType::Codex => crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title.as_deref().and_then(clean_title)),
        AgentType::Claude => crate::claude_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title.as_deref().and_then(clean_title)),
        AgentType::Gemini => crate::gemini_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title.as_deref().and_then(clean_title)),
        _ => None,
    }
}

fn resolve_live_panel_subtitle(panel: &AgentPanel) -> Option<String> {
    panel.last_user_prompt.clone().or_else(|| {
        panel
            .cached_preview_turns
            .first()
            .map(|turn| turn.question.clone())
    })
}

fn file_mtime(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::{build_codex_history_entry, merge_or_insert_thread};
    use crate::codex_state::CodexThreadRef;
    use crate::model::{AgentState, AgentType, PreviewTurn, SessionCacheState};
    use crate::session_cache::SessionCacheSnapshot;
    use crate::sidebar::model::{SidebarFolder, SidebarThread};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn folder() -> SidebarFolder {
        SidebarFolder {
            key: "/repo".into(),
            path: "/repo".into(),
            label: "repo".into(),
            updated_at: 0,
            threads: Vec::new(),
        }
    }

    fn codex_thread() -> CodexThreadRef {
        CodexThreadRef {
            thread_id: "sid-1".into(),
            cwd: PathBuf::from("/repo"),
            updated_at: 42,
            rollout_path: PathBuf::from("/repo/.codex/sid-1.jsonl"),
            title: Some("upstream title".into()),
            first_user_message: Some("old first prompt".into()),
            source: None,
            archived: false,
        }
    }

    #[test]
    fn codex_history_prefers_session_cache_prompt_for_subtitle() {
        let thread = build_codex_history_entry(
            &folder(),
            &codex_thread(),
            Some(&SessionCacheSnapshot {
                agent_session_id: "sid-1".into(),
                transcript_path: Some("/repo/.codex/sid-1.jsonl".into()),
                recent_turns: vec![PreviewTurn {
                    question: "newest prompt".into(),
                    answer: Some("answer".into()),
                }],
                last_user_prompt: Some("newest prompt".into()),
                last_assistant_message: Some("answer".into()),
                state: SessionCacheState::Cached,
            }),
        );

        assert_eq!(thread.subtitle.as_deref(), Some("newest prompt"));
        assert_eq!(thread.last_user_prompt.as_deref(), Some("newest prompt"));
        assert_eq!(thread.cached_preview_turns.len(), 1);
    }

    #[test]
    fn merge_or_insert_preserves_history_prompt_when_live_thread_lacks_one() {
        let mut threads = vec![SidebarThread {
            key: "live:%1".into(),
            folder_key: "/repo".into(),
            working_dir: "/repo".into(),
            folder_label: "repo".into(),
            agent_type: AgentType::Codex,
            runtime_source: None,
            session_id: Some("sid-1".into()),
            transcript_path: Some("/repo/.codex/sid-1.jsonl".into()),
            title: "live".into(),
            upstream_title: None,
            subtitle: None,
            title_override: None,
            note: None,
            tags: Vec::new(),
            pinned: false,
            updated_at: 1,
            sort_updated_at: 1,
            live_pane_id: Some("%1".into()),
            live_location: None,
            pid: None,
            git_info: None,
            state: AgentState::Idle,
            is_active: false,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
            archived: false,
        }];

        let history = build_codex_history_entry(
            &folder(),
            &codex_thread(),
            Some(&SessionCacheSnapshot {
                agent_session_id: "sid-1".into(),
                transcript_path: Some("/repo/.codex/sid-1.jsonl".into()),
                recent_turns: vec![PreviewTurn {
                    question: "newest prompt".into(),
                    answer: None,
                }],
                last_user_prompt: Some("newest prompt".into()),
                last_assistant_message: None,
                state: SessionCacheState::Cached,
            }),
        );

        merge_or_insert_thread(&mut threads, history, &[], &HashMap::new());

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].subtitle.as_deref(), Some("newest prompt"));
        assert_eq!(
            threads[0].last_user_prompt.as_deref(),
            Some("newest prompt")
        );
        assert_eq!(threads[0].cached_preview_turns.len(), 1);
        assert_eq!(
            threads[0].session_cache_state,
            Some(SessionCacheState::Cached)
        );
    }
}
