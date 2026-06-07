use crate::app::App;
use crate::log_debug;
use crate::relay;
use crate::session;
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
                    relay::apply_runtime_configs(
                        &app.config.agents,
                        &app.config.agent_permissions,
                        &app.config.codex,
                    );
                    let agent_cmd = match crate::codex_runtime::prepare_agent_command(
                        &agent_name,
                        &raw_agent_cmd,
                    ) {
                        Ok(command) => command,
                        Err(err) => {
                            log_debug!(
                                "agent_launcher: prepare command failed name={} cmd={} err={}",
                                agent_name,
                                raw_agent_cmd,
                                err
                            );
                            return;
                        }
                    };
                    log_debug!(
                        "agent_launcher: launching cmd={} dir={}",
                        agent_cmd,
                        target_dir.display()
                    );

                    app.close_agent_launcher();
                    app.dirty = true;

                    if from_fuzzy {
                        let dir_str = target_dir.to_string_lossy().to_string();
                        let cmd = agent_cmd.clone();
                        if !app.saved_tmux_bindings.is_empty() || app.same_session_attached {
                            crate::event::restore_tmux_bindings(app);
                            app.same_session_attached = false;
                        }
                        log_debug!(
                            "agent_launcher: from_fuzzy=true, create_session_with_agent dir={} cmd={}",
                            dir_str,
                            cmd
                        );
                        match session::create_session_with_agent(app, &dir_str, &cmd) {
                            Ok(()) => log_debug!("agent_launcher: create_session_with_agent 成功"),
                            Err(e) => {
                                log_debug!("agent_launcher: create_session_with_agent 失败: {}", e)
                            }
                        }
                    } else {
                        std::thread::spawn(move || {
                            if matches!(agent_cmd.trim(), "gemini" | "gemini-cli") {
                                let target_dir = target_dir.to_string_lossy().to_string();
                                if let Ok(out) = std::process::Command::new("tmux")
                                    .args([
                                        "new-window",
                                        "-P",
                                        "-F",
                                        "#{pane_id}",
                                        "-c",
                                        &target_dir,
                                    ])
                                    .output()
                                {
                                    if out.status.success() {
                                        let pane_id =
                                            String::from_utf8_lossy(&out.stdout).trim().to_string();
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

                    app.schedule_delayed_scan(800);
                }
            }
            _ => {}
        }
    }
}
