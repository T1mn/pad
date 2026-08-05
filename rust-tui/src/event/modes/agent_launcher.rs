use std::path::{Path, PathBuf};

use crate::app::{state::Mode, App};
use crate::log_debug;
use crate::relay;
use crate::session;
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
        "agent_launcher: launching runtime={} name={} cmd={} dir={}",
        if app.runtime_mode.uses_tmux() {
            "tmux"
        } else {
            "native"
        },
        agent_name,
        agent_cmd,
        target_dir.display()
    );

    app.close_agent_launcher();
    app.dirty = true;

    if !app.runtime_mode.uses_tmux() {
        launch_native_agent(app, &agent_name, &agent_cmd, target_dir);
        return;
    }

    launch_tmux_agent(app, from_fuzzy, target_dir, agent_cmd);
    app.schedule_delayed_scan(800);
}

fn launch_native_agent(app: &mut App, agent_name: &str, agent_cmd: &str, target_dir: PathBuf) {
    app.sidebar.show_tree = false;
    app.mode = Mode::Normal;
    let size = app
        .focused_terminal_pane()
        .and_then(|pane| pane.size())
        .unwrap_or_else(|| TerminalSize::new(80, 24));
    let label = native_agent_label(agent_name, &target_dir);
    match app.launch_native_agent_terminal_at(&label, agent_cmd, target_dir, size) {
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

fn launch_tmux_agent(app: &mut App, from_fuzzy: bool, target_dir: PathBuf, agent_cmd: String) {
    if from_fuzzy {
        let dir_str = target_dir.to_string_lossy().to_string();
        if !app.saved_tmux_bindings.is_empty() || app.same_session_attached {
            crate::event::restore_tmux_bindings(app);
            app.same_session_attached = false;
        }
        log_debug!(
            "agent_launcher: from_fuzzy=true, create_session_with_agent dir={} cmd={}",
            dir_str,
            agent_cmd
        );
        match session::create_session_with_agent(app, &dir_str, &agent_cmd) {
            Ok(()) => log_debug!("agent_launcher: create_session_with_agent 成功"),
            Err(error) => {
                log_debug!("agent_launcher: create_session_with_agent 失败: {}", error)
            }
        }
        return;
    }

    std::thread::spawn(move || {
        if matches!(agent_cmd.trim(), "gemini" | "gemini-cli") {
            let target_dir = target_dir.to_string_lossy().to_string();
            if let Ok(out) = std::process::Command::new("tmux")
                .args(["new-window", "-P", "-F", "#{pane_id}", "-c", &target_dir])
                .output()
            {
                if out.status.success() {
                    let pane_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    let script = format!(
                        "sleep 0.2; tmux send-keys -t '{}' C-c; tmux send-keys -t '{}' 'clear' Enter; tmux send-keys -t '{}' '{}' Enter",
                        pane_id, pane_id, pane_id, agent_cmd
                    );
                    let _ = std::process::Command::new("tmux")
                        .args(["run-shell", "-b", &script])
                        .output();
                }
            }
        } else {
            let _ = std::process::Command::new("tmux")
                .args(["new-window", "-c", &target_dir.to_string_lossy()])
                .arg(&agent_cmd)
                .spawn();
        }
    });
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
mod tests;
