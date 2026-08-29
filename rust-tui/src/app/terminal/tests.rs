use std::path::PathBuf;

use std::sync::Arc;

use crate::terminal_runtime::{
    EngineId, PaneFrame, PaneId, PaneMetadata, TerminalScroll, TerminalSize, TerminalSnapshot,
    TransportId,
};

use super::controller_io::PendingTerminalIo;
use super::*;

fn cwd() -> PathBuf {
    PathBuf::from("/tmp/pad-terminal-tests")
}

pub(crate) fn nested_splits_close_and_collapse_without_orphans() {
    let mut workspace = TerminalWorkspace::default();
    let shell = workspace.add_tab(TerminalProfile::Shell, cwd()).unwrap();
    let codex = workspace
        .split_focused(TerminalSplitAxis::Columns, TerminalProfile::Codex, cwd())
        .unwrap();
    let claude = workspace
        .split_focused(TerminalSplitAxis::Rows, TerminalProfile::Claude, cwd())
        .unwrap();

    assert_eq!(workspace.visible_pane_ids(), vec![shell, codex, claude]);
    assert_eq!(workspace.focused_pane_id(), Some(claude));
    workspace.validate().unwrap();

    assert!(workspace.focus_pane(codex));
    assert!(workspace.close_pane(codex));
    assert_eq!(workspace.visible_pane_ids(), vec![shell, claude]);
    assert_eq!(workspace.focused_pane_id(), Some(claude));
    assert!(workspace.pane(codex).is_none());
    workspace.validate().unwrap();

    assert!(workspace.close_pane(claude));
    assert_eq!(workspace.visible_pane_ids(), vec![shell]);
    assert!(matches!(
        workspace.active_tab().map(|tab| &tab.root),
        Some(TerminalLayoutNode::Pane { pane_id }) if *pane_id == shell
    ));
    workspace.validate().unwrap();
}

pub(crate) fn tabs_remember_focus_and_clamp_after_close() {
    let mut workspace = TerminalWorkspace::default();
    let first = workspace.add_tab(TerminalProfile::Shell, cwd()).unwrap();
    let second = workspace
        .split_focused(TerminalSplitAxis::Columns, TerminalProfile::Codex, cwd())
        .unwrap();
    assert!(workspace.focus_pane(first));

    let third = workspace
        .add_tab(TerminalProfile::GithubCli, cwd())
        .unwrap();
    assert_eq!(workspace.active_tab, 1);
    assert_eq!(workspace.focused_pane_id(), Some(third));
    assert!(workspace.cycle_tab(-1));
    assert_eq!(workspace.focused_pane_id(), Some(first));
    assert!(!workspace.focus_pane(first));
    assert!(workspace.cycle_pane(1));
    assert_eq!(workspace.focused_pane_id(), Some(second));

    assert!(workspace.close_pane(first));
    assert_eq!(workspace.focused_pane_id(), Some(second));
    assert!(workspace.close_pane(second));
    assert_eq!(workspace.tabs.len(), 1);
    assert_eq!(workspace.active_tab, 0);
    assert_eq!(workspace.focused_pane_id(), Some(third));

    let fourth = workspace.add_tab(TerminalProfile::Codex, cwd()).unwrap();
    let fifth = workspace
        .split_focused(TerminalSplitAxis::Columns, TerminalProfile::Claude, cwd())
        .unwrap();
    let removed = workspace.close_tab(1).unwrap();
    assert_eq!(removed, vec![fourth, fifth]);
    assert_eq!(workspace.tabs.len(), 1);
    assert!(workspace.pane(fourth).is_none());
    assert!(workspace.pane(fifth).is_none());
    assert_eq!(workspace.focused_pane_id(), Some(third));
    workspace.validate().unwrap();
}

pub(crate) fn pane_ids_are_monotonic_across_close_and_restore() {
    let mut workspace = TerminalWorkspace::default();
    let first = workspace.add_tab(TerminalProfile::Shell, cwd()).unwrap();
    let second = workspace.add_tab(TerminalProfile::Codex, cwd()).unwrap();
    assert!(workspace.close_pane(second));
    let third = workspace.add_tab(TerminalProfile::Claude, cwd()).unwrap();
    assert!(third.serial() > second.serial());

    workspace.next_pane_serial = 1;
    workspace.normalize_after_restore().unwrap();
    let fourth = workspace
        .add_tab(TerminalProfile::GithubCli, cwd())
        .unwrap();
    assert!(fourth.serial() > third.serial());
    assert!(first.serial() < fourth.serial());
}

pub(crate) fn workspace_json_contains_only_stable_layout_and_launch_data() {
    let mut workspace = TerminalWorkspace::default();
    let shell = workspace.add_tab(TerminalProfile::Shell, cwd()).unwrap();
    let codex = workspace
        .split_focused(TerminalSplitAxis::Columns, TerminalProfile::Codex, cwd())
        .unwrap();
    assert!(workspace.rename_pane(codex, "Review".to_string()));

    let json = serde_json::to_string(&workspace).unwrap();
    assert!(!json.contains("epoch"));
    assert!(!json.contains("frame"));
    assert!(!json.contains("runtime_id"));
    assert!(!json.contains("\"command\""));

    let mut restored: TerminalWorkspace = serde_json::from_str(&json).unwrap();
    restored.normalize_after_restore().unwrap();
    assert_eq!(restored, workspace);
    assert_eq!(restored.pane(shell).unwrap().command.program, None);
    assert_eq!(
        restored.pane(codex).unwrap().command.program.as_deref(),
        Some("codex")
    );

    let mut legacy = TerminalWorkspace::default();
    let legacy_codex = legacy.add_tab(TerminalProfile::Shell, cwd()).unwrap();
    assert!(legacy.rename_pane(legacy_codex, "Codex · pad-terminal-tests".into()));
    legacy.normalize_after_restore().unwrap();
    assert_eq!(
        legacy.pane(legacy_codex).unwrap().profile,
        TerminalProfile::Codex
    );
}

pub(crate) fn all_builtin_profiles_have_deterministic_commands() {
    let cases = [
        (TerminalProfile::Shell, None),
        (TerminalProfile::Codex, Some("codex")),
        (TerminalProfile::Claude, Some("claude")),
        (TerminalProfile::Pi, Some("pi")),
        (TerminalProfile::OpenCode, Some("opencode")),
        (TerminalProfile::GithubCli, None),
    ];
    for (profile, expected) in cases {
        assert_eq!(profile.default_command().program.as_deref(), expected);
    }
    assert_eq!(
        TerminalProfile::Pi.default_command().args,
        vec!["--mode".to_string(), "rpc".to_string()]
    );
    assert_eq!(
        agent_registry::native_profile_agent_type(TerminalProfile::OpenCode),
        Some(AgentType::OpenCode)
    );
    assert_eq!(
        agent_registry::native_profile_agent_type(TerminalProfile::Shell),
        None
    );
    assert_eq!(
        agent_registry::native_agent_terminal_profile(&AgentType::Codex),
        TerminalProfile::Codex
    );
    assert_eq!(
        agent_registry::native_agent_terminal_profile(&AgentType::Claude),
        TerminalProfile::Claude
    );
    assert_eq!(
        agent_registry::native_agent_terminal_profile(&AgentType::OpenCode),
        TerminalProfile::OpenCode
    );
    assert_eq!(
        agent_registry::native_profile_agent_type(TerminalProfile::Pi),
        Some(AgentType::Pi)
    );
    assert_eq!(
        agent_registry::native_agent_terminal_profile(&AgentType::Pi),
        TerminalProfile::Pi
    );
}

pub(crate) fn labels_are_trimmed_and_control_characters_are_rejected() {
    assert_eq!(
        model::normalize_label("  Agent review  ").unwrap(),
        "Agent review"
    );
    assert!(model::normalize_label(" ").is_err());
    assert!(model::normalize_label("bad\nlabel").is_err());
    assert!(model::normalize_label(&"x".repeat(121)).is_err());
}

pub(crate) fn configured_agent_is_the_direct_pty_child_of_a_known_shell() {
    assert_eq!(
        native_agent_command("opencode --model 'fast'"),
        TerminalCommandDefinition {
            program: Some("/bin/sh".into()),
            args: vec!["-lc".into(), "opencode --model 'fast'".into()],
        }
    );
}

pub(crate) fn exited_native_agent_rejects_later_prompt_input() {
    let mut app = App::new();
    let pane_id = app
        .terminal
        .workspace
        .add_tab(TerminalProfile::Shell, cwd())
        .unwrap();
    let definition = app.terminal.workspace.pane(pane_id).unwrap().clone();
    app.terminal
        .install_pane_runtime(&definition, TerminalSize::new(20, 5));
    app.terminal.panes.get_mut(&pane_id).unwrap().lifecycle = TerminalPaneLifecycle::Exited;

    let error = app
        .send_native_pane_input(
            &agent_registry::native_agent_pane_id(pane_id),
            b"do not run in a shell\r".to_vec(),
        )
        .unwrap_err();

    assert!(error.to_string().contains("not running"));
    assert!(app
        .terminal
        .panes
        .get(&pane_id)
        .unwrap()
        .pending_io
        .is_empty());
}

pub(crate) fn closing_an_earlier_tab_reindexes_native_sidebar_entries() {
    let mut app = App::new();
    let first = app
        .terminal
        .workspace
        .add_tab(TerminalProfile::OpenCode, cwd())
        .unwrap();
    let second = app
        .terminal
        .workspace
        .add_tab(TerminalProfile::Codex, cwd())
        .unwrap();
    app.register_native_agent_panel(first, AgentType::OpenCode, "OpenCode".into(), cwd());
    app.register_native_agent_panel(second, AgentType::Codex, "Codex".into(), cwd());
    assert_eq!(app.panels[1].window_index, "2");

    assert!(app.close_terminal_pane(first).unwrap());

    assert_eq!(app.panels.len(), 1);
    assert_eq!(app.panels[0].pane_id, format!("native:{}", second.serial()));
    assert_eq!(app.panels[0].window_index, "1");
}

pub(crate) fn native_pane_focus_keeps_sidebar_selection_on_the_same_agent() {
    let mut app = App::new();
    let first = app
        .terminal
        .workspace
        .add_tab(TerminalProfile::OpenCode, cwd())
        .unwrap();
    let second = app
        .terminal
        .workspace
        .add_tab(TerminalProfile::Codex, cwd())
        .unwrap();
    app.register_native_agent_panel(first, AgentType::OpenCode, "OpenCode".into(), cwd());
    app.register_native_agent_panel(second, AgentType::Codex, "Codex".into(), cwd());

    app.mark_native_agent_panel_active(first);

    let expected = format!("live:{}", agent_registry::native_agent_pane_id(first));
    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some(expected.as_str())
    );
}

pub(crate) fn closing_the_last_terminal_pane_leaves_an_empty_workspace() {
    let mut app = App::new();
    let only = app
        .terminal
        .workspace
        .add_tab(TerminalProfile::OpenCode, cwd())
        .unwrap();
    app.register_native_agent_panel(only, AgentType::OpenCode, "OpenCode".into(), cwd());

    assert!(app.close_terminal_pane(only).unwrap());

    assert!(app.terminal.workspace.panes.is_empty());
    assert!(app.terminal.workspace.tabs.is_empty());
    assert!(app.panels.is_empty());
    assert_eq!(app.focused_terminal_pane_id(), None);
}

pub(crate) fn failed_workspace_save_rolls_back_a_close_before_runtime_mutation() {
    let mut app = App::new();
    let first = app
        .terminal
        .workspace
        .add_tab(TerminalProfile::OpenCode, cwd())
        .unwrap();
    let second = app
        .terminal
        .workspace
        .add_tab(TerminalProfile::Codex, cwd())
        .unwrap();
    app.register_native_agent_panel(first, AgentType::OpenCode, "OpenCode".into(), cwd());
    app.register_native_agent_panel(second, AgentType::Codex, "Codex".into(), cwd());
    runtime_io::fail_next_workspace_save();

    let error = app.close_terminal_pane(first).unwrap_err();

    assert!(error.to_string().contains("injected"));
    assert!(app.terminal.workspace.pane(first).is_some());
    assert!(app.terminal.workspace.pane(second).is_some());
    assert_eq!(app.panels.len(), 2);
}

pub(crate) fn keyboard_input_queues_bottom_before_bytes_but_mouse_does_not() {
    let mut state = TerminalUiState::default();
    let pane_id = state
        .workspace
        .add_tab(TerminalProfile::Shell, cwd())
        .unwrap();
    let definition = state.workspace.pane(pane_id).unwrap().clone();
    state.install_pane_runtime(&definition, TerminalSize::new(20, 5));
    let pane = state.panes.get_mut(&pane_id).unwrap();
    let mut terminal = TerminalSnapshot::blank(TerminalSize::new(20, 5));
    terminal.viewport.display_offset = 3;
    terminal.viewport.history_size = 10;
    pane.frame = Some(Arc::new(PaneFrame {
        metadata: PaneMetadata {
            id: PaneId::new("native-1"),
            label: "Shell".to_string(),
            engine_id: EngineId::new("alacritty"),
            transport_id: TransportId::new("native:1"),
        },
        terminal,
    }));

    state.queue_input(pane_id, b"key".to_vec(), true).unwrap();
    let pane = state.panes.get(&pane_id).unwrap();
    assert!(matches!(
        pane.pending_io.front(),
        Some(PendingTerminalIo::Scroll(TerminalScroll::Bottom))
    ));
    assert!(matches!(
        pane.pending_io.get(1),
        Some(PendingTerminalIo::Input(bytes)) if bytes == b"key"
    ));
    state.queue_input(pane_id, b"2".to_vec(), true).unwrap();
    let pane = state.panes.get(&pane_id).unwrap();
    assert_eq!(
        pane.pending_io
            .iter()
            .filter(|io| matches!(io, PendingTerminalIo::Scroll(TerminalScroll::Bottom)))
            .count(),
        1
    );

    let mut mouse_state = TerminalUiState::default();
    let mouse_id = mouse_state
        .workspace
        .add_tab(TerminalProfile::Shell, cwd())
        .unwrap();
    let definition = mouse_state.workspace.pane(mouse_id).unwrap().clone();
    mouse_state.install_pane_runtime(&definition, TerminalSize::new(20, 5));
    mouse_state
        .queue_input(mouse_id, b"mouse".to_vec(), false)
        .unwrap();
    assert!(matches!(
        mouse_state
            .panes
            .get(&mouse_id)
            .unwrap()
            .pending_io
            .front(),
        Some(PendingTerminalIo::Input(bytes)) if bytes == b"mouse"
    ));
}

pub(crate) fn resize_and_scroll_state_are_independent_per_pane() {
    let mut state = TerminalUiState::default();
    let first = state
        .workspace
        .add_tab(TerminalProfile::Shell, cwd())
        .unwrap();
    let second = state
        .workspace
        .add_tab(TerminalProfile::Codex, cwd())
        .unwrap();
    for pane_id in [first, second] {
        let definition = state.workspace.pane(pane_id).unwrap().clone();
        state.install_pane_runtime(&definition, TerminalSize::new(20, 5));
    }

    state.queue_resize(first, TerminalSize::new(40, 10));
    state.queue_scroll(second, TerminalScroll::PageUp).unwrap();

    assert_eq!(
        state.panes.get(&first).unwrap().pending_resize,
        Some(TerminalSize::new(40, 10))
    );
    assert!(state.panes.get(&first).unwrap().pending_io.is_empty());
    assert!(matches!(
        state.panes.get(&second).unwrap().pending_io.front(),
        Some(PendingTerminalIo::Scroll(TerminalScroll::PageUp))
    ));
    assert_eq!(state.panes.get(&second).unwrap().pending_resize, None);
}

pub(crate) fn pending_scroll_is_reset_before_immediate_keyboard_input() {
    let mut state = TerminalUiState::default();
    let pane_id = state
        .workspace
        .add_tab(TerminalProfile::Shell, cwd())
        .unwrap();
    let definition = state.workspace.pane(pane_id).unwrap().clone();
    state.install_pane_runtime(&definition, TerminalSize::new(20, 5));

    state.queue_scroll(pane_id, TerminalScroll::PageUp).unwrap();
    // Simulate the controller accepting PageUp before it publishes a frame.
    state.panes.get_mut(&pane_id).unwrap().pending_io.clear();
    state.queue_input(pane_id, b"key".to_vec(), true).unwrap();

    let pending = &state.panes.get(&pane_id).unwrap().pending_io;
    assert!(matches!(
        pending.front(),
        Some(PendingTerminalIo::Scroll(TerminalScroll::Bottom))
    ));
    assert!(matches!(
        pending.get(1),
        Some(PendingTerminalIo::Input(bytes)) if bytes == b"key"
    ));
}

pub(crate) fn scroll_queue_coalesces_lines_and_has_a_hard_limit() {
    let mut state = TerminalUiState::default();
    let pane_id = state
        .workspace
        .add_tab(TerminalProfile::Shell, cwd())
        .unwrap();
    let definition = state.workspace.pane(pane_id).unwrap().clone();
    state.install_pane_runtime(&definition, TerminalSize::new(20, 5));

    state
        .queue_scroll(pane_id, TerminalScroll::Lines(3))
        .unwrap();
    state
        .queue_scroll(pane_id, TerminalScroll::Lines(4))
        .unwrap();
    assert!(matches!(
        state.panes.get(&pane_id).unwrap().pending_io.front(),
        Some(PendingTerminalIo::Scroll(TerminalScroll::Lines(7)))
    ));

    state.panes.get_mut(&pane_id).unwrap().pending_io.clear();
    for _ in 0..MAX_PENDING_SCROLL_COMMANDS {
        state.queue_scroll(pane_id, TerminalScroll::PageUp).unwrap();
    }
    assert!(state.queue_scroll(pane_id, TerminalScroll::PageUp).is_err());
    state.queue_scroll(pane_id, TerminalScroll::Bottom).unwrap();
    state.queue_scroll(pane_id, TerminalScroll::Bottom).unwrap();
    assert_eq!(
        state
            .panes
            .get(&pane_id)
            .unwrap()
            .pending_io
            .iter()
            .filter(|io| matches!(io, PendingTerminalIo::Scroll(_)))
            .count(),
        1
    );
}

pub(crate) fn scroll_reset_keeps_barriers_for_already_queued_input() {
    let mut state = TerminalUiState::default();
    let pane_id = state
        .workspace
        .add_tab(TerminalProfile::Shell, cwd())
        .unwrap();
    let definition = state.workspace.pane(pane_id).unwrap().clone();
    state.install_pane_runtime(&definition, TerminalSize::new(20, 5));

    state.queue_scroll(pane_id, TerminalScroll::Bottom).unwrap();
    state.queue_input(pane_id, b"A".to_vec(), true).unwrap();
    state.queue_scroll(pane_id, TerminalScroll::PageUp).unwrap();
    state.queue_input(pane_id, b"B".to_vec(), true).unwrap();

    let pending = &state.panes.get(&pane_id).unwrap().pending_io;
    assert_eq!(pending.len(), 4);
    assert!(matches!(
        pending.front(),
        Some(PendingTerminalIo::Scroll(TerminalScroll::Bottom))
    ));
    assert!(matches!(
        pending.get(1),
        Some(PendingTerminalIo::Input(bytes)) if bytes == b"A"
    ));
    assert!(matches!(
        pending.get(2),
        Some(PendingTerminalIo::Scroll(TerminalScroll::Bottom))
    ));
    assert!(matches!(
        pending.get(3),
        Some(PendingTerminalIo::Input(bytes)) if bytes == b"B"
    ));
}

pub(crate) fn restored_serial_exhaustion_is_rejected_without_panicking() {
    let mut workspace = TerminalWorkspace::default();
    workspace.add_tab(TerminalProfile::Shell, cwd()).unwrap();
    workspace.next_pane_serial = u64::MAX;
    assert!(workspace.normalize_after_restore().is_err());

    let exhausted = TerminalPaneId::new(u64::MAX - 1);
    workspace.next_pane_serial = 2;
    workspace.panes[0].id = exhausted;
    workspace.tabs[0].root = TerminalLayoutNode::pane(exhausted);
    workspace.tabs[0].focused = exhausted;
    assert!(workspace.normalize_after_restore().is_err());
}

pub(crate) fn restored_commands_are_derived_from_profiles() {
    let mut workspace = TerminalWorkspace::default();
    let pane_id = workspace.add_tab(TerminalProfile::Codex, cwd()).unwrap();
    workspace.panes[0].command = TerminalCommandDefinition::program("malicious-helper");

    workspace.normalize_after_restore().unwrap();
    assert_eq!(
        workspace.pane(pane_id).unwrap().command,
        TerminalProfile::Codex.default_command()
    );
}

pub(crate) fn rename_stays_bound_to_the_pane_that_started_it() {
    let mut app = App::new();
    let first = app
        .terminal
        .workspace
        .add_tab(TerminalProfile::Shell, cwd())
        .unwrap();
    let second = app
        .terminal
        .workspace
        .split_focused(TerminalSplitAxis::Columns, TerminalProfile::Shell, cwd())
        .unwrap();
    assert!(app.terminal.workspace.focus_pane(first));
    assert!(app.begin_terminal_rename());
    assert!(app.terminal.workspace.focus_pane(second));
    app.clear_terminal_rename();
    app.append_terminal_rename_text("Pinned name");
    app.commit_terminal_rename().unwrap();

    assert_eq!(
        app.terminal.workspace.pane(first).unwrap().label,
        "Pinned name"
    );
    assert_ne!(
        app.terminal.workspace.pane(second).unwrap().label,
        "Pinned name"
    );
}

pub(crate) fn split_rejects_geometry_that_cannot_render_both_children() {
    assert!(validate_split_size(TerminalSplitAxis::Columns, TerminalSize::new(9, 24)).is_err());
    assert!(validate_split_size(TerminalSplitAxis::Rows, TerminalSize::new(80, 5)).is_err());
    validate_split_size(TerminalSplitAxis::Columns, TerminalSize::new(10, 6)).unwrap();
    validate_split_size(TerminalSplitAxis::Rows, TerminalSize::new(10, 6)).unwrap();
}

#[cfg(unix)]
pub(crate) fn restarting_the_same_app_relaunches_the_retained_workspace() {
    let mut app = App::new();
    app.start_native_terminal(TerminalSize::new(40, 10))
        .unwrap();
    let snapshot = app.terminal_workspace_snapshot();
    assert_eq!(app.terminal.panes.len(), snapshot.panes.len());

    app.shutdown_native_terminal().unwrap();
    assert!(app.terminal.panes.is_empty());
    app.start_native_terminal(TerminalSize::new(40, 10))
        .unwrap();

    assert_eq!(app.terminal_workspace(), &snapshot);
    assert_eq!(app.terminal.panes.len(), snapshot.panes.len());
    app.shutdown_native_terminal().unwrap();
}

#[cfg(unix)]
pub(crate) fn builtin_opencode_profile_registers_and_restores_its_live_sidebar_entry() {
    crate::test_support::with_temp_home("pad-terminal-profile", "opencode-registry", |home| {
        let cwd = home.join("project");
        std::fs::create_dir_all(&cwd).unwrap();
        let mut app = App::new();
        app.start_native_terminal(TerminalSize::new(40, 10))
            .unwrap();
        let pane_id = app
            .create_terminal_tab_at(
                TerminalProfile::OpenCode,
                cwd.clone(),
                TerminalSize::new(40, 10),
            )
            .unwrap();

        assert_eq!(app.panels.len(), 1);
        assert_eq!(app.panels[0].agent_type, AgentType::OpenCode);
        assert_eq!(app.panels[0].working_dir, cwd.to_string_lossy());
        assert_eq!(
            app.panels[0].pane_id,
            format!("native:{}", pane_id.serial())
        );

        let snapshot = app.terminal_workspace_snapshot();
        app.shutdown_native_terminal().unwrap();

        let mut restored = App::new();
        restored
            .restore_native_terminal_workspace(snapshot, TerminalSize::new(40, 10))
            .unwrap();
        assert_eq!(restored.panels.len(), 1);
        assert_eq!(restored.panels[0].agent_type, AgentType::OpenCode);
        assert_eq!(restored.panels[0].working_dir, cwd.to_string_lossy());
        restored.shutdown_native_terminal().unwrap();
    });
}

#[cfg(unix)]
pub(crate) fn builtin_opencode_profile_uses_the_full_configured_shell_command() {
    use std::os::unix::fs::PermissionsExt;

    crate::test_support::with_temp_home("pad-terminal-profile", "opencode-command", |home| {
        let bin_dir = home.join(".opencode/bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let command = bin_dir.join("opencode");
        std::fs::write(
            &command,
            "#!/bin/sh\nprintf '%s' \"$*\" > \"$HOME/opencode-profile-args\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cwd = home.join("project");
        std::fs::create_dir_all(&cwd).unwrap();

        let mut app = App::new();
        app.config
            .agents
            .iter_mut()
            .find(|agent| agent.name == "opencode")
            .unwrap()
            .cmd = "~/.opencode/bin/opencode --pure".into();
        app.start_native_terminal(TerminalSize::new(40, 10))
            .unwrap();
        app.create_terminal_tab_at(TerminalProfile::OpenCode, cwd, TerminalSize::new(40, 10))
            .unwrap();

        let marker = home.join("opencode-profile-args");
        for _ in 0..100 {
            app.poll_native_terminal();
            if marker.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(std::fs::read_to_string(marker).unwrap(), "--pure");
        app.shutdown_native_terminal().unwrap();
    });
}
