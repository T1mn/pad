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
