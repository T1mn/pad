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
                    let agent_cmd = agent.1.to_string();
                    log_debug!(
                        "agent_launcher: launching cmd={} dir={}",
                        agent_cmd,
                        target_dir.display()
                    );

                    app.close_agent_launcher();
                    app.dirty = true;

                    relay::apply_relay_configs(&app.config.agents);

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
                            let _ = std::process::Command::new("tmux")
                                .args(["new-window", "-c", &target_dir.to_string_lossy()])
                                .arg(&agent_cmd)
                                .spawn();
                        });
                    }

                    app.schedule_delayed_scan(800);
                }
            }
            _ => {}
        }
    }
}
