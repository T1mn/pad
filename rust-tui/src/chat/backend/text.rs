use crate::model::AgentPanel;
use crate::sidebar::clean_title;
use std::collections::VecDeque;
use std::path::Path;

const PANE_CAPTURE_SUMMARY_LINES: usize = 18;

pub(crate) fn build_slash_command_text(command: &str, arg: &str) -> String {
    if arg.is_empty() {
        command.to_string()
    } else {
        format!("{} {}", command, arg.trim())
    }
}

pub(crate) fn summarize_pane_capture(text: &str) -> String {
    let mut tail = VecDeque::with_capacity(PANE_CAPTURE_SUMMARY_LINES);
    let mut pending_blank_lines = 0usize;

    for line in text.lines().map(str::trim_end) {
        if line.trim().is_empty() {
            if !tail.is_empty() {
                pending_blank_lines += 1;
            }
            continue;
        }

        for _ in 0..pending_blank_lines {
            push_summary_line(&mut tail, "");
        }
        pending_blank_lines = 0;
        push_summary_line(&mut tail, line);
    }

    join_summary_lines(tail)
}

fn push_summary_line<'a>(tail: &mut VecDeque<&'a str>, line: &'a str) {
    if tail.len() == PANE_CAPTURE_SUMMARY_LINES {
        tail.pop_front();
    }
    tail.push_back(line);
}

fn join_summary_lines(lines: VecDeque<&str>) -> String {
    let mut summary = String::new();
    for (idx, line) in lines.into_iter().enumerate() {
        if idx > 0 {
            summary.push('\n');
        }
        summary.push_str(line);
    }
    summary
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
