use super::*;

#[cfg(unix)]
#[test]
fn native_launcher_opens_selected_agent_in_a_real_terminal_tab() {
    crate::test_support::with_temp_home("pad-agent-launcher", "native-opencode", |home| {
        let target = home.join("project");
        std::fs::create_dir_all(&target).unwrap();
        let marker = home.join("native-opencode-launched");
        let command = format!(
            "printf native-opencode-launch > {}",
            crate::codex_runtime::shell_single_quote(&marker.to_string_lossy())
        );

        let mut app = App::new();
        app.start_native_terminal(TerminalSize::new(80, 24))
            .unwrap();
        app.open_agent_launcher(target.clone());
        app.fuzzy_from_normal = true;
        let launcher = app.sidebar.agent_launcher.as_mut().unwrap();
        launcher.agents = vec![("opencode".to_string(), command)];
        launcher.selected = 0;

        handle_agent_launcher_mode(&mut app, KeyCode::Enter);

        assert!(matches!(app.mode, Mode::Normal));
        assert!(app.terminal_is_focused());
        assert_eq!(app.terminal_workspace().tabs.len(), 2);
        assert_eq!(app.terminal_workspace().panes.len(), 2);
        let agent_pane_id = app.focused_terminal_pane_id().unwrap();
        let pane = app.focused_terminal_pane().unwrap();
        assert_eq!(pane.cwd(), target);
        assert_eq!(pane.label(), "OpenCode · project");
        assert_eq!(app.panels.len(), 1);
        assert_eq!(app.panels[0].agent_type, AgentType::OpenCode);
        assert_eq!(app.panels[0].working_dir, target.to_string_lossy());
        assert!(App::is_native_agent_terminal_id(&app.panels[0].pane_id));
        assert_eq!(
            app.sidebar.selected_sidebar_key.as_deref(),
            Some(format!("live:{}", app.panels[0].pane_id).as_str())
        );
        assert!(app
            .sidebar
            .expanded_folders
            .contains(target.to_string_lossy().as_ref()));
        assert_eq!(
            app.selected_preview_thread().unwrap().title,
            "OpenCode · project"
        );
        for _ in 0..100 {
            if marker.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(
            std::fs::read_to_string(marker).unwrap(),
            "native-opencode-launch"
        );
        let panel = app.panels[0].clone();
        app.delete_panel(&panel);
        assert!(app.terminal_pane(agent_pane_id).is_none());
        assert!(app.panels.is_empty());
        app.shutdown_native_terminal().unwrap();
    });
}

#[test]
fn native_agent_labels_are_human_readable() {
    assert_eq!(
        native_agent_label("opencode", Path::new("/tmp/repo")),
        "OpenCode · repo"
    );
    assert_eq!(
        native_agent_label("claude-code", Path::new("/tmp/repo")),
        "Claude · repo"
    );
}
