use crate::model::AgentPanel;
use crate::runtime_status;
use crate::scanner;
use crate::session_cache;
use std::io;
use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const PANEL_CACHE_TTL: Duration = Duration::from_millis(800);

struct CachedPanels {
    loaded_at: Instant,
    panels: Vec<AgentPanel>,
}

static PANEL_CACHE: LazyLock<Mutex<Option<CachedPanels>>> = LazyLock::new(|| Mutex::new(None));

pub(crate) fn latest_answer_for_pane(pane_id: &str) -> Option<String> {
    let mut panels = scanner::scan_panels().ok()?;
    let _ = session_cache::preload_panels(&mut panels);
    let panel = panels.into_iter().find(|panel| panel.pane_id == pane_id)?;
    panel
        .last_assistant_message
        .filter(|text| !text.trim().is_empty())
        .or_else(|| {
            panel
                .cached_preview_turns
                .first()
                .and_then(|turn| turn.answer.clone())
                .filter(|text| !text.trim().is_empty())
        })
}

pub(crate) fn live_panels() -> Result<Vec<AgentPanel>, Box<dyn std::error::Error>> {
    if let Ok(cache) = PANEL_CACHE.lock() {
        if let Some(cache) = cache.as_ref() {
            if cache.loaded_at.elapsed() <= PANEL_CACHE_TTL {
                return Ok(cache.panels.clone());
            }
        }
    }

    let mut panels = scanner::scan_panels().map_err(|err| io::Error::other(err.to_string()))?;
    let _ = session_cache::preload_panels(&mut panels);
    if let Ok(mut cache) = PANEL_CACHE.lock() {
        *cache = Some(CachedPanels {
            loaded_at: Instant::now(),
            panels: panels.clone(),
        });
    }
    Ok(panels)
}

pub(crate) fn invalidate_live_panels() {
    if let Ok(mut cache) = PANEL_CACHE.lock() {
        *cache = None;
    }
}

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

pub(crate) fn pad_is_online() -> bool {
    runtime_status::read_status(&crate::paths::pad_status_path())
        .map(|status| runtime_status::process_alive(status.pid))
        .unwrap_or(false)
}

pub(crate) fn compact_target_label(panel: &AgentPanel) -> String {
    format!(
        "{} • {}",
        panel.agent_type.to_string().to_uppercase(),
        leaf_name(&panel.working_dir)
    )
}
