use crate::model::AgentPanel;
use crate::runtime_status;
use crate::scanner;
use crate::session_cache;
use crate::sidebar::clean_title;
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

pub(crate) fn pad_is_online() -> bool {
    runtime_status::read_status(&crate::paths::pad_status_path())
        .map(|status| runtime_status::process_alive(status.pid))
        .unwrap_or(false)
}

pub(crate) fn compact_target_label(panel: &AgentPanel) -> String {
    format!(
        "{} • {}",
        panel.agent_type.to_string().to_uppercase(),
        panel_display_title(panel)
    )
}

#[cfg(test)]
mod tests {
    use super::{leaf_name, panel_display_title};
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn temp_home(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("pad-chat-backend-{name}-{stamp}"))
    }

    fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
        let _guard = test_lock().lock().expect("lock backend tests");
        let home = temp_home(name);
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("create temp home");

        let prev_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &home);

        let result = f(&home);

        if let Some(prev) = prev_home {
            std::env::set_var("HOME", prev);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = std::fs::remove_dir_all(&home);

        result
    }

    fn sample_panel(session_id: Option<&str>) -> AgentPanel {
        AgentPanel {
            session: "0".into(),
            window: "zsh".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%42".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/rust-tui".into(),
            is_active: false,
            state: AgentState::Idle,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: session_id.map(str::to_string),
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        }
    }

    #[test]
    fn panel_display_title_uses_thread_meta_title_override() {
        with_temp_home("title-override", |_| {
            crate::thread_meta::upsert_thread_meta(
                "codex",
                "session-1",
                Some("  Renamed title  \nignored line"),
                None,
                false,
            )
            .expect("write thread meta");

            let panel = sample_panel(Some("session-1"));
            assert_eq!(panel_display_title(&panel), "Renamed title");
        });
    }

    #[test]
    fn panel_display_title_falls_back_to_working_dir_leaf() {
        let panel = sample_panel(None);
        assert_eq!(panel_display_title(&panel), leaf_name(&panel.working_dir));
    }
}
