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
        app.runtime_mode = crate::runtime_mode::RuntimeMode::Native;
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
        assert!(app.delayed_scan_at.is_none());
        let pane = app.focused_terminal_pane().unwrap();
        assert_eq!(pane.cwd(), target);
        assert_eq!(pane.label(), "OpenCode · project");
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
