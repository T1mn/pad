mod panels {
    use crate::model::AgentPanel;
    use crate::model::{AgentState, AgentType};
    use crate::socket_api::model::ApiRequest;
    use std::io;
    use std::sync::{LazyLock, Mutex};
    use std::time::{Duration, Instant};

    const PANEL_CACHE_TTL: Duration = Duration::from_millis(800);

    struct CachedPanels {
        loaded_at: Instant,
        panels: Vec<AgentPanel>,
    }

    static PANEL_CACHE: LazyLock<Mutex<Option<CachedPanels>>> = LazyLock::new(|| Mutex::new(None));

    pub(crate) fn live_panels() -> Result<Vec<AgentPanel>, Box<dyn std::error::Error>> {
        if let Ok(cache) = PANEL_CACHE.lock() {
            if let Some(cache) = cache.as_ref() {
                if cache.loaded_at.elapsed() <= PANEL_CACHE_TTL {
                    return Ok(cache.panels.clone());
                }
            }
        }

        let response = crate::socket_api::client::send_request(&ApiRequest {
            action: "status".to_string(),
            ..ApiRequest::default()
        })?;
        if !response.ok {
            return Err(io::Error::other(response.message).into());
        }
        let panels: Vec<AgentPanel> = response
            .data
            .as_ref()
            .and_then(|data| data.get("panels"))
            .and_then(serde_json::Value::as_array)
            .map(|panels| panels.iter().filter_map(panel_from_json).collect())
            .unwrap_or_default();
        if let Ok(mut cache) = PANEL_CACHE.lock() {
            *cache = Some(CachedPanels {
                loaded_at: Instant::now(),
                panels: panels.clone(),
            });
        }
        Ok(panels)
    }

    fn panel_from_json(value: &serde_json::Value) -> Option<AgentPanel> {
        let text = |key: &str| value.get(key)?.as_str().map(str::to_string);
        let agent_name = text("agent_type")?;
        let state = match text("state")?.as_str() {
            "busy" => AgentState::Busy,
            "waiting" => AgentState::Waiting,
            _ => AgentState::Idle,
        };
        Some(AgentPanel {
            session: text("session").unwrap_or_else(|| "native".to_string()),
            window: text("window").unwrap_or_else(|| agent_name.clone()),
            window_index: text("window_index").unwrap_or_default(),
            pane: text("pane").unwrap_or_default(),
            pane_id: text("pane_id")?,
            agent_type: AgentType::from_processes(&agent_name),
            working_dir: text("working_dir")?,
            is_active: value
                .get("is_active")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            state,
            transcript_path: text("transcript_path"),
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            agent_session_id: text("agent_session_id"),
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        })
    }

    pub(crate) fn invalidate_live_panels() {
        if let Ok(mut cache) = PANEL_CACHE.lock() {
            *cache = None;
        }
    }
}
mod status {
    use crate::runtime_status;

    pub(crate) fn pad_is_online() -> bool {
        runtime_status::read_status(&crate::paths::pad_status_path())
            .map(|status| runtime_status::process_alive(status.pid))
            .unwrap_or(false)
    }
}
mod text {
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
}

pub(crate) use panels::{invalidate_live_panels, live_panels};
pub(crate) use status::pad_is_online;
pub(crate) use text::{
    build_slash_command_text, compact_target_label, panel_display_title, summarize_pane_capture,
};

#[cfg(test)]
use text::leaf_name;

#[cfg(test)]
mod tests {
    use super::{leaf_name, panel_display_title, summarize_pane_capture};
    use crate::model::{AgentPanel, AgentState, AgentType};
    use std::path::Path;

    fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
        crate::test_support::with_temp_home("pad-chat-backend", name, f)
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
            transcript_path: None,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
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

    #[test]
    fn summarize_pane_capture_trims_outer_blank_lines_and_keeps_tail() {
        let text = format!(
            "\n  \n{}\n\n",
            (1..=20)
                .map(|idx| format!("line {idx}   "))
                .collect::<Vec<_>>()
                .join("\n")
        );

        assert_eq!(
            summarize_pane_capture(&text),
            (3..=20)
                .map(|idx| format!("line {idx}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn summarize_pane_capture_preserves_inner_blank_lines() {
        assert_eq!(
            summarize_pane_capture("\nfirst  \n   \nsecond\n\n"),
            "first\n\nsecond"
        );
    }
}
