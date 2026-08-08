mod archived {
    use crate::model::{AgentPanel, AgentType};

    pub(in crate::sidebar::build) fn should_hide_live_panel(panel: &AgentPanel) -> bool {
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
}
mod fallback {
    use super::super::meta::apply_thread_metadata;
    use super::thread_from_live_panel;
    use crate::model::AgentPanel;
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::super::super::display::folder_display_label;
    use super::super::super::model::SidebarFolder;
    use super::super::super::sort::{folder_sort_key, thread_sort_key};

    pub(in crate::sidebar::build) fn build_live_panel_fallback_folders(
        panels: &[AgentPanel],
    ) -> Vec<SidebarFolder> {
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
            folder.threads.push(Arc::new(thread_from_live_panel(panel)));
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
}
mod resolve {
    use crate::model::{AgentPanel, AgentType};
    use std::path::Path;

    use super::super::super::display::clean_title;

    pub(super) fn resolve_live_panel_upstream_title(panel: &AgentPanel) -> Option<String> {
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

    pub(super) fn resolve_live_panel_subtitle(panel: &AgentPanel) -> Option<String> {
        panel.last_user_prompt.clone().or_else(|| {
            panel
                .cached_preview_turns
                .first()
                .map(|turn| turn.question.clone())
        })
    }

    pub(super) fn resolve_live_panel_session_provider_name(panel: &AgentPanel) -> Option<String> {
        if let Some(path) = panel.transcript_path.as_deref() {
            return crate::sidebar::provider::resolve_session_provider_name(
                &panel.agent_type,
                Some(Path::new(path)),
            );
        }

        if panel.agent_type == AgentType::Codex {
            let session_id = panel.agent_session_id.as_deref()?;
            let thread = crate::codex_state::thread_for_id(session_id)
                .ok()
                .flatten()?;
            return crate::sidebar::provider::resolve_session_provider_name(
                &panel.agent_type,
                Some(thread.rollout_path.as_path()),
            );
        }

        None
    }
}
mod thread {
    use super::super::meta::{apply_thread_meta, load_thread_meta_for_panel};
    use super::resolve::{
        resolve_live_panel_session_provider_name, resolve_live_panel_subtitle,
        resolve_live_panel_upstream_title,
    };
    use crate::model::AgentPanel;
    use std::path::Path;

    use super::super::super::display::folder_display_label;
    use super::super::super::model::SidebarThread;

    pub fn thread_from_live_panel(panel: &AgentPanel) -> SidebarThread {
        let updated_at = panel
            .transcript_path
            .as_deref()
            .and_then(|path| file_mtime(Path::new(path)))
            .unwrap_or_default();
        let upstream_title = resolve_live_panel_upstream_title(panel);
        let subtitle = resolve_live_panel_subtitle(panel);
        let fallback_title = panel.agent_session_id.as_deref().or_else(|| {
            (panel.session == "native" && !panel.window.is_empty())
                .then_some(panel.window.as_str())
                .or(Some(panel.pane_id.as_str()))
        });
        let mut thread = SidebarThread {
            key: format!("live:{}", panel.pane_id),
            folder_key: panel.working_dir.clone(),
            working_dir: panel.working_dir.clone(),
            folder_label: folder_display_label(&panel.working_dir),
            agent_type: panel.agent_type.clone(),
            session_id: panel.agent_session_id.clone(),
            transcript_path: panel.transcript_path.clone(),
            session_provider_name: resolve_live_panel_session_provider_name(panel),
            title: upstream_title
                .clone()
                .or_else(|| fallback_title.map(|value| value.to_string()))
                .unwrap_or_else(|| "untitled".to_string()),
            upstream_title,
            generated_title: None,
            subtitle,
            title_override: None,
            note: None,
            share_url: None,
            cost: None,
            token_summary: None,
            tags: Vec::new(),
            pinned: false,
            updated_at,
            sort_updated_at: 0,
            live_pane_id: Some(panel.pane_id.clone()),
            live_location: Some(format!(
                "{}:{}.{}",
                panel.session, panel.window_index, panel.pane
            )),
            state: panel.state.clone(),
            is_active: panel.is_active,
            cached_preview_turns: panel.cached_preview_turns.clone(),
            session_cache_state: panel.session_cache_state,
            last_user_prompt: panel.last_user_prompt.clone(),
            last_assistant_message: panel.last_assistant_message.clone(),
            has_unread_stop: panel.has_unread_stop,
            archived: false,
            deleted: false,
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

    fn file_mtime(path: &Path) -> Option<i64> {
        std::fs::metadata(path)
            .ok()?
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs() as i64)
    }
}

pub(super) use archived::should_hide_live_panel;
pub(super) use fallback::build_live_panel_fallback_folders;
pub use thread::thread_from_live_panel;
