use super::*;

#[test]
fn space_on_selected_thread_collapses_parent_folder() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();
    app.select_sidebar_index(1, false);

    let mut terminal = test_terminal();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();
    app.flush_pending_sidebar_space_action();

    assert!(!app.sidebar.expanded_folders.contains("/tmp/alpha"));
    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some("/tmp/alpha")
    );
    assert_eq!(app.table_state.selected(), Some(0));
    assert!(app.preview.focus == FocusTarget::Panel);
}
#[test]
fn double_space_expands_all_folders_when_none_are_expanded() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sync_sidebar_selection();

    let mut terminal = test_terminal();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();

    assert!(app.sidebar.expanded_folders.contains("/tmp/alpha"));
    assert!(app.sidebar.expanded_folders.contains("/tmp/beta"));
}
#[test]
fn double_space_collapses_all_folders_when_any_are_expanded() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.sidebar.expanded_folders.insert("/tmp/beta".into());
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();
    app.select_sidebar_index(1, false);

    let mut terminal = test_terminal();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char(' '))).unwrap();

    assert!(app.sidebar.expanded_folders.is_empty());
    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some("/tmp/alpha")
    );
    assert_eq!(app.table_state.selected(), Some(0));
}
#[test]
fn j_k_skip_expanded_folder_rows() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();

    let mut terminal = test_terminal();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char('j'))).unwrap();
    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));

    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char('k'))).unwrap();
    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some("/tmp/beta")
    );
}
#[test]
fn numeric_jump_targets_visible_threads_only() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();

    let mut terminal = test_terminal();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char('1'))).unwrap();
    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));

    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Char('2'))).unwrap();
    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));
}

#[cfg(unix)]
#[test]
fn enter_on_native_agent_thread_focuses_its_terminal_tab() {
    crate::test_support::with_temp_home("pad-sidebar", "native-agent-enter", |home| {
        let target = home.join("project");
        std::fs::create_dir_all(&target).unwrap();
        let mut app = App::new();
        app.runtime_mode = crate::runtime_mode::RuntimeMode::Native;
        app.start_native_terminal(crate::terminal_runtime::TerminalSize::new(80, 24))
            .unwrap();
        let native_pane = app
            .launch_native_agent_terminal_at(
                "OpenCode · project",
                "true",
                AgentType::OpenCode,
                target,
                crate::terminal_runtime::TerminalSize::new(80, 24),
            )
            .unwrap();

        assert!(app.focus_terminal_tab(0));
        app.focus_panel();
        let mut terminal = test_terminal();
        handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Enter)).unwrap();

        assert_eq!(app.focused_terminal_pane_id(), Some(native_pane));
        assert!(app.terminal_is_focused());
        app.shutdown_native_terminal().unwrap();
    });
}
