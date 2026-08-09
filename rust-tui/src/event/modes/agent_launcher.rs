use std::path::{Path, PathBuf};

use crate::app::{state::Mode, App};
use crate::log_debug;
use crate::model::AgentType;
use crate::relay;
use crate::terminal_runtime::TerminalSize;
use crossterm::event::KeyCode;

pub(crate) fn handle_agent_launcher_mode(app: &mut App, key: KeyCode) {
    let from_fuzzy = app.fuzzy_from_normal;

    if let Some(ref mut launcher) = app.sidebar.agent_launcher {
        log_debug!(
            "agent_launcher key={:?} selected={} from_fuzzy={}",
            key,
            launcher.selected,
            from_fuzzy
        );
        match key {
            KeyCode::Esc => {
                app.close_agent_launcher();
                app.dirty = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                launcher.next();
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                launcher.previous();
                app.dirty = true;
            }
            KeyCode::Enter => {
                if let Some(agent) = launcher.selected_agent() {
                    let target_dir = launcher.target_dir.clone();
                    let agent_name = agent.0.to_string();
                    let raw_agent_cmd = agent.1.to_string();
                    launch_selected_agent(app, from_fuzzy, target_dir, agent_name, raw_agent_cmd);
                }
            }
            _ => {}
        }
    }
}

fn launch_selected_agent(
    app: &mut App,
    from_fuzzy: bool,
    target_dir: PathBuf,
    agent_name: String,
    raw_agent_cmd: String,
) {
    // Only apply the selected agent's config.
    if let Some(selected_agent) = app
        .config
        .agents
        .iter()
        .find(|agent| agent.name == agent_name)
    {
        relay::apply_relay_configs(std::slice::from_ref(selected_agent));
    }

    let agent_cmd = match crate::codex_runtime::prepare_agent_command(&agent_name, &raw_agent_cmd) {
        Ok(command) => command,
        Err(error) => {
            log_debug!(
                "agent_launcher: prepare command failed name={} cmd={} err={}",
                agent_name,
                raw_agent_cmd,
                error
            );
            app.show_action_toast("Agent launch failed", &error.to_string());
            return;
        }
    };
    log_debug!(
        "agent_launcher: launching runtime=native name={} cmd={} dir={}",
        agent_name,
        agent_cmd,
        target_dir.display()
    );

    app.close_agent_launcher();
    app.dirty = true;

    let _ = from_fuzzy;
    launch_native_agent(app, &agent_name, &agent_cmd, target_dir);
}

fn launch_native_agent(app: &mut App, agent_name: &str, agent_cmd: &str, target_dir: PathBuf) {
    app.sidebar.show_tree = false;
    app.mode = Mode::Normal;
    let size = app
        .focused_terminal_pane()
        .and_then(|pane| pane.size())
        .unwrap_or_else(|| TerminalSize::new(80, 24));
    let label = native_agent_label(agent_name, &target_dir);
    let agent_type = AgentType::from_processes(agent_name);
    match app.launch_native_agent_terminal_at(&label, agent_cmd, agent_type, target_dir, size) {
        Ok(_) => {
            app.focus_terminal();
            log_debug!("agent_launcher: native agent terminal opened");
        }
        Err(error) => {
            log_debug!("agent_launcher: native launch failed: {}", error);
            app.show_action_toast("Agent launch failed", &error.to_string());
        }
    }
}

fn native_agent_label(agent_name: &str, target_dir: &Path) -> String {
    let display_name = match agent_name.trim().to_ascii_lowercase().as_str() {
        "claude" | "claude-code" => "Claude",
        "codex" => "Codex",
        "opencode" => "OpenCode",
        "gemini" | "gemini-cli" => "Gemini",
        "grok" | "grok-build" => "Grok",
        _ => agent_name.trim(),
    };
    match target_dir.file_name().and_then(|name| name.to_str()) {
        Some(directory) if !directory.is_empty() => format!("{display_name} · {directory}"),
        _ => display_name.to_string(),
    }
}

#[cfg(test)]
#[path = "agent_launcher_tests.rs"]
pub(crate) mod tests;
