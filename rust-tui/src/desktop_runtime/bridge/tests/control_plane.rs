use super::*;
use crate::permission_policy::{Profile, Task};

#[cfg(unix)]
pub(crate) fn terminal_v2_bridge_is_task_bound_bounded_and_redacted() {
    let workspace = std::env::temp_dir().join(format!(
        "pad-terminal-bridge-{}-{}",
        std::process::id(),
        crate::time::unix_now_nanos()
    ));
    std::fs::create_dir_all(&workspace).unwrap();
    let mut runtime = runtime();
    runtime.set_terminal_program_for_test(PathBuf::from("/bin/sh"));
    runtime
        .store_mut()
        .insert_task(&Task {
            id: "task-terminal".to_string(),
            profile_id: "profile-bridge".to_string(),
            title: "Terminal".to_string(),
            cwd: workspace,
            ..Default::default()
        })
        .unwrap();

    let open = json!({
        "id": "terminal-open",
        "action": "terminal_open",
        "protocol_version": 2,
        "task_id": "task-terminal",
        "pane_id": "pane-1",
        "columns": 80,
        "rows": 24,
    });
    let (open, _) = handle_line(&mut runtime, &open.to_string());
    assert!(open.ok, "terminal open failed: {open:?}");
    let open = open.result.unwrap();
    assert_eq!(open["pane_id"], "pane-1");
    assert_eq!(open["task_id"], "task-terminal");
    assert_eq!(open["status"], "opening");

    let input = json!({
        "id": "terminal-input",
        "action": "terminal_input",
        "protocol_version": 2,
        "pane_id": "pane-1",
        "data": "printf 'terminal-ok PI_SESSION_FILE=/Users/tim/.codex/private\\n'\n",
    });
    let (input, _) = handle_line(&mut runtime, &input.to_string());
    assert!(input.ok, "terminal input failed: {input:?}");

    let resize = json!({
        "id": "terminal-resize",
        "action": "terminal_resize",
        "protocol_version": 2,
        "pane_id": "pane-1",
        "columns": 120,
        "rows": 40,
    });
    let (resize, _) = handle_line(&mut runtime, &resize.to_string());
    assert!(resize.ok, "terminal resize failed: {resize:?}");
    assert_eq!(
        resize.result.unwrap()["size"],
        json!({"columns": 120, "rows": 40})
    );

    let snapshot = json!({
        "id": "terminal-snapshot",
        "action": "terminal_snapshot",
        "protocol_version": 2,
        "pane_id": "pane-1",
    });
    let mut encoded = String::new();
    let mut last = None;
    for _ in 0..200 {
        let (response, _) = handle_line(&mut runtime, &snapshot.to_string());
        assert!(response.ok, "terminal snapshot failed: {response:?}");
        encoded = serde_json::to_string(&response).unwrap();
        last = response.result;
        if encoded.contains("terminal-ok") {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        encoded.contains("terminal-ok"),
        "terminal output missing: {encoded}"
    );
    assert!(!encoded.contains("PI_SESSION_FILE"));
    assert!(!encoded.contains(".codex"));
    let last = last.unwrap();
    assert!(last["lines"].as_array().unwrap().len() <= 40);
    assert_eq!(last["size"], json!({"columns": 120, "rows": 40}));
    assert!(last.get("cwd").is_none());
    assert!(last.get("env").is_none());

    let oversized_input = DesktopRequest {
        action: Some("terminal_input".to_string()),
        protocol_version: Some(2),
        pane_id: Some("pane-1".to_string()),
        data: Some("x".repeat(crate::desktop_runtime::terminal::MAX_TERMINAL_INPUT_BYTES + 1)),
        ..Default::default()
    };
    let error = handle_request(&mut runtime, &oversized_input).unwrap_err();
    assert_eq!(error.code, "terminal_input_too_large");

    let invalid_size = DesktopRequest {
        action: Some("terminal_resize".to_string()),
        protocol_version: Some(2),
        pane_id: Some("pane-1".to_string()),
        columns: Some(crate::desktop_runtime::terminal::MAX_TERMINAL_COLUMNS + 1),
        rows: Some(24),
        ..Default::default()
    };
    let error = handle_request(&mut runtime, &invalid_size).unwrap_err();
    assert_eq!(error.code, "invalid_terminal_size");

    let close = json!({
        "id": "terminal-close",
        "action": "terminal_close",
        "protocol_version": 2,
        "pane_id": "pane-1",
    });
    let (close, _) = handle_line(&mut runtime, &close.to_string());
    assert!(close.ok, "terminal close failed: {close:?}");
    assert!(close.result.unwrap()["closed"].as_bool().unwrap());
}

pub(crate) fn v2_server_events_cover_task_runtime_account_and_auth() {
    let runtime = runtime();
    let response = DesktopResponse {
        id: Some("event".to_string()),
        ok: true,
        result: Some(json!({"auth": {"phase": "running"}})),
        error: None,
    };
    let mut sequence = 0;
    let cases = [
        (
            "poll",
            "task-bridge",
            vec!["task_changed", "runtime_changed"],
        ),
        ("set_profile", "", vec!["account_changed"]),
        ("auth_status", "", vec!["auth_changed"]),
    ];
    for (action, task_id, expected) in cases {
        let request = DesktopRequest {
            action: Some(action.to_string()),
            task_id: (!task_id.is_empty()).then(|| task_id.to_string()),
            protocol_version: Some(2),
            ..Default::default()
        };
        let events = events_after_request(&runtime, &request, &response, &mut sequence);
        let kinds = events
            .iter()
            .filter_map(|event| event["event"]["kind"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(kinds, expected);
    }
    assert_eq!(sequence, 4);
}

pub(crate) fn ui_state_v2_normalizes_references_and_drives_sidebar_snapshot() {
    use crate::pad_store::{DesktopTheme, SidebarWidth};
    use crate::permission_policy::{Project, Section, SectionItem};

    let mut runtime = runtime();
    runtime
        .store_mut()
        .insert_profile(&Profile {
            id: "profile-state".to_string(),
            name: "State profile".to_string(),
            agent_dir: std::env::temp_dir().join("pad-state-agent"),
            session_dir: std::env::temp_dir().join("pad-state-sessions"),
            default_provider: Some("state-provider".to_string()),
            default_model: Some("state-model".to_string()),
            ..Default::default()
        })
        .unwrap();
    runtime
        .store_mut()
        .insert_project(&Project {
            id: "project-state".to_string(),
            name: "State project".to_string(),
            primary_root: std::env::temp_dir(),
            profile_id: Some("profile-state".to_string()),
            ..Default::default()
        })
        .unwrap();
    for (id, project_id) in [
        ("task-state-selected", None),
        ("task-state-child", Some("project-state")),
    ] {
        runtime
            .store_mut()
            .insert_task(&Task {
                id: id.to_string(),
                project_id: project_id.map(str::to_string),
                profile_id: "profile-state".to_string(),
                title: id.to_string(),
                cwd: std::env::temp_dir(),
                ..Default::default()
            })
            .unwrap();
    }
    runtime
        .store_mut()
        .insert_section(&Section {
            id: "section-state".to_string(),
            name: "State section".to_string(),
            items: vec![SectionItem::Project("project-state".to_string())],
            ..Default::default()
        })
        .unwrap();

    let state = DesktopUiState {
        active_profile_id: Some("profile-state".to_string()),
        selected_task_id: Some("task-state-child".to_string()),
        collapsed_section_ids: vec![
            "section:section-state".to_string(),
            "synthetic-section-key".to_string(),
        ],
        collapsed_project_ids: vec!["project:project-state".to_string()],
        sidebar_width: SidebarWidth::new(320).unwrap(),
        sidebar_view: crate::pad_store::DesktopSidebarView::All,
        theme: DesktopTheme::Dark,
        right_panel_open: true,
        bottom_panel_open: true,
        sidebar_open: true,
    };
    let request = json!({
        "id": "set-ui-state",
        "action": "set_ui_state",
        "protocol_version": 2,
        "state": state,
    });
    let (response, _) = handle_line(&mut runtime, &request.to_string());
    assert!(response.ok, "set_ui_state failed: {response:?}");
    let result = response.result.unwrap();
    assert_eq!(result["state"]["active_profile_id"], "profile-state");
    assert_eq!(result["state"]["selected_task_id"], "task-state-child");
    assert_eq!(result["state"]["sidebar_width"], 320);
    assert_eq!(result["state"]["sidebar_view"], "all");
    assert_eq!(result["state"]["theme"], "dark");
    assert!(result["state"]["collapsed_section_ids"]
        .as_array()
        .unwrap()
        .contains(&json!("synthetic-section-key")));
    assert_eq!(result["sidebar"]["active_profile_id"], "profile-state");
    assert_eq!(result["sidebar"]["view"], "all");
    assert_eq!(result["sidebar"]["selected_key"], "task:task-state-child");
    let row_keys = result["sidebar"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["key"].as_str())
        .collect::<Vec<_>>();
    assert!(row_keys.contains(&"section:section-state"));
    assert!(row_keys.contains(&"project:project-state"));
    assert!(!row_keys.contains(&"task:task-state-child"));

    for action in ["bootstrap", "list_sidebar", "get_ui_state"] {
        let request = json!({
            "id": format!("ui-{action}"),
            "action": action,
            "protocol_version": 2,
        });
        let (response, _) = handle_line(&mut runtime, &request.to_string());
        assert!(response.ok, "{action} failed: {response:?}");
        let result = response.result.unwrap();
        let returned = if action == "get_ui_state" {
            &result["state"]
        } else {
            &result["ui_state"]
        };
        assert_eq!(returned["active_profile_id"], "profile-state");
        assert_eq!(returned["selected_task_id"], "task-state-child");
        if action == "bootstrap" {
            assert_eq!(result["profile"]["id"], "profile-state");
            assert_eq!(result["backend"]["selected_provider"], "state-provider");
            assert_eq!(result["backend"]["selected_model"], "state-model");
        }
    }

    let cross_profile = json!({
        "id": "cross-profile-ui-state",
        "action": "set_ui_state",
        "protocol_version": 2,
        "state": {
            "active_profile_id": "profile-bridge",
            "selected_task_id": "task-state-selected",
            "collapsed_section_ids": [],
            "collapsed_project_ids": [],
            "sidebar_width": 275,
            "theme": "system",
            "right_panel_open": false,
            "bottom_panel_open": false,
            "sidebar_open": true
        }
    });
    let (cross_profile, _) = handle_line(&mut runtime, &cross_profile.to_string());
    assert_eq!(cross_profile.error.unwrap().code, "task_not_found");

    let missing_profile = json!({
        "id": "missing-profile-ui-state",
        "action": "set_ui_state",
        "protocol_version": 2,
        "state": {
            "active_profile_id": "missing-profile",
            "selected_task_id": "task-state-selected",
            "collapsed_section_ids": [],
            "collapsed_project_ids": [],
            "sidebar_width": 275,
            "theme": "system",
            "right_panel_open": false,
            "bottom_panel_open": false,
            "sidebar_open": true
        }
    });
    let (missing_profile, _) = handle_line(&mut runtime, &missing_profile.to_string());
    assert_eq!(missing_profile.error.unwrap().code, "profile_not_active");

    let nested_unknown = json!({
        "id": "unknown-ui-state-field",
        "action": "set_ui_state",
        "protocol_version": 2,
        "state": {
            "active_profile_id": null,
            "selected_task_id": null,
            "collapsed_section_ids": [],
            "collapsed_project_ids": [],
            "sidebar_width": 275,
            "theme": "system",
            "right_panel_open": false,
            "bottom_panel_open": false,
            "sidebar_open": true,
            "credential": "must-not-be-accepted"
        }
    });
    let (nested_unknown, _) = handle_line(&mut runtime, &nested_unknown.to_string());
    assert_eq!(nested_unknown.error.unwrap().code, "invalid_request");

    let top_level_unknown = json!({
        "id": "unknown-ui-state-top-level",
        "action": "get_ui_state",
        "protocol_version": 2,
        "unexpected": true,
    });
    let (top_level_unknown, _) = handle_line(&mut runtime, &top_level_unknown.to_string());
    assert_eq!(top_level_unknown.error.unwrap().code, "invalid_fields");

    let (legacy, _) = handle_line(
        &mut runtime,
        r#"{"id":"legacy-ui","action":"get_ui_state"}"#,
    );
    assert_eq!(legacy.error.unwrap().code, "protocol_upgrade_required");
}

pub(crate) fn ui_state_v2_survives_runtime_restart() {
    use crate::pad_store::{DesktopTheme, PadStore, SidebarWidth};

    let root = std::env::temp_dir().join(format!(
        "pad-ui-state-restart-{}-{}",
        std::process::id(),
        crate::time::unix_now_nanos()
    ));
    let database = root.join("pad.sqlite");
    std::fs::create_dir_all(&root).unwrap();
    {
        let mut runtime = DesktopRuntime::from_store(PadStore::open(&database).unwrap());
        runtime
            .store_mut()
            .insert_profile(&Profile {
                id: "profile-restart".to_string(),
                name: "Restart".to_string(),
                agent_dir: root.join("agent"),
                session_dir: root.join("sessions"),
                ..Default::default()
            })
            .unwrap();
        runtime
            .store_mut()
            .insert_task(&Task {
                id: "task-restart".to_string(),
                profile_id: "profile-restart".to_string(),
                title: "Restart".to_string(),
                cwd: std::env::temp_dir(),
                ..Default::default()
            })
            .unwrap();
        runtime
            .set_desktop_ui_state(DesktopUiState {
                active_profile_id: Some("profile-restart".to_string()),
                selected_task_id: Some("task-restart".to_string()),
                collapsed_section_ids: vec!["synthetic:recent".to_string()],
                collapsed_project_ids: Vec::new(),
                sidebar_width: SidebarWidth::new(333).unwrap(),
                sidebar_view: crate::pad_store::DesktopSidebarView::Archive,
                theme: DesktopTheme::Light,
                right_panel_open: true,
                bottom_panel_open: false,
                sidebar_open: true,
            })
            .unwrap();
    }
    let mut runtime = DesktopRuntime::from_store(PadStore::open(&database).unwrap());
    let request = json!({
        "id": "restart-ui-state",
        "action": "get_ui_state",
        "protocol_version": 2,
    });
    let (response, _) = handle_line(&mut runtime, &request.to_string());
    assert!(
        response.ok,
        "get_ui_state after restart failed: {response:?}"
    );
    let state = response.result.unwrap()["state"].clone();
    assert_eq!(state["active_profile_id"], "profile-restart");
    assert_eq!(state["selected_task_id"], "task-restart");
    assert_eq!(state["sidebar_width"], 333);
    assert_eq!(state["sidebar_view"], "archive");
    assert_eq!(state["theme"], "light");
    assert!(state["right_panel_open"].as_bool().unwrap());
}

#[cfg(unix)]
pub(crate) fn rust_auth_control_plane_owns_prompt_response_and_secret() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let mut runtime = runtime();
    let root = std::env::temp_dir().join(format!(
        "pad-auth-bridge-{}-{}",
        std::process::id(),
        crate::time::unix_now_nanos()
    ));
    let package_root = root.join("pi-package");
    fs::create_dir_all(&package_root).unwrap();
    fs::write(package_root.join("package.json"), b"{}\n").unwrap();
    let helper = root.join("auth-helper");
    fs::write(
        &helper,
        concat!(
            "#!/bin/sh\n",
            "printf '%s\\n' '{\"type\":\"prompt\",\"id\":\"prompt-1\",\"kind\":\"secret\",\"message\":\"API key\",\"options\":[]}'\n",
            "IFS= read -r response\n",
            "case \"$response\" in *super-secret-value*) ;; *) exit 2 ;; esac\n",
            "printf '%s\\n' '{\"type\":\"success\",\"provider\":\"test-provider\"}'\n",
        ),
    )
    .unwrap();
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
    runtime.set_auth_launcher_for_test(helper, package_root);

    let begin = json!({
        "id": "auth-begin",
        "action": "auth_begin",
        "protocol_version": 2,
        "profile_id": "profile-bridge",
        "provider": "test-provider",
        "auth_type": "api_key",
    });
    let (begin, _) = handle_line(&mut runtime, &begin.to_string());
    assert!(begin.ok);
    let attempt_id = begin.result.unwrap()["auth"]["attempt_id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut prompt_id = None;
    for _ in 0..100 {
        let request = json!({
            "id": "auth-status",
            "action": "auth_status",
            "protocol_version": 2,
            "attempt_id": attempt_id,
        });
        let (status, _) = handle_line(&mut runtime, &request.to_string());
        prompt_id = status
            .result
            .as_ref()
            .and_then(|result| result["auth"]["prompt"]["id"].as_str())
            .map(str::to_string);
        if prompt_id.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let prompt_id = prompt_id.expect("auth helper did not emit a prompt");
    let respond = json!({
        "id": "auth-respond",
        "action": "auth_respond",
        "protocol_version": 2,
        "attempt_id": attempt_id,
        "prompt_id": prompt_id,
        "value": "super-secret-value",
    });
    let (respond, _) = handle_line(&mut runtime, &respond.to_string());
    assert!(respond.ok);
    assert!(!serde_json::to_string(&respond)
        .unwrap()
        .contains("super-secret-value"));

    let mut phase = String::new();
    for _ in 0..100 {
        let request = json!({
            "id": "auth-status",
            "action": "auth_status",
            "protocol_version": 2,
            "attempt_id": attempt_id,
        });
        let (status, _) = handle_line(&mut runtime, &request.to_string());
        phase = status.result.unwrap()["auth"]["phase"]
            .as_str()
            .unwrap()
            .to_string();
        if phase == "succeeded" {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(phase, "succeeded");
}
