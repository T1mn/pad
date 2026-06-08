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
    let lines = text.lines().map(str::trim_end).collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| !line.trim().is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map_or(start, |idx| idx + 1);
    let lines = &lines[start..end];
    let tail_start = lines.len().saturating_sub(18);
    lines[tail_start..].join("\n")
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
