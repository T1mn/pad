use crate::model::AgentPanel;
use crate::sidebar::clean_title;
use std::path::Path;

pub(crate) fn build_slash_command_text(command: &str, arg: &str) -> String {
    if arg.is_empty() {
        command.to_string()
    } else {
        format!("{} {}", command, arg.trim())
    }
}

pub(crate) fn summarize_pane_capture(text: &str) -> String {
    let mut lines = text.lines().map(str::trim_end).collect::<Vec<_>>();
    while matches!(lines.first(), Some(line) if line.trim().is_empty()) {
        lines.remove(0);
    }
    while matches!(lines.last(), Some(line) if line.trim().is_empty()) {
        lines.pop();
    }
    if lines.len() > 18 {
        lines = lines[lines.len().saturating_sub(18)..].to_vec();
    }
    lines.join("\n")
}

pub(crate) fn leaf_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

fn title_override_for_panel(panel: &AgentPanel) -> Option<String> {
    let session_id = panel.agent_session_id.as_deref()?;
    let meta = crate::thread_meta::load_thread_meta(&panel.agent_type.to_string(), session_id)
        .ok()
        .flatten()?;
    meta.title_override.as_deref().and_then(clean_title)
}

pub(crate) fn panel_display_title(panel: &AgentPanel) -> String {
    title_override_for_panel(panel).unwrap_or_else(|| leaf_name(&panel.working_dir))
}

pub(crate) fn compact_target_label(panel: &AgentPanel) -> String {
    format!(
        "{} • {}",
        panel.agent_type.to_string().to_uppercase(),
        panel_display_title(panel)
    )
}
