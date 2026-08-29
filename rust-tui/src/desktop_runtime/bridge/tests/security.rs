use super::*;
use crate::permission_policy::{Profile, Task};

pub(crate) fn protocol_v2_bootstrap_uses_renderer_safe_records() {
    let mut runtime = runtime();
    let mut profile = runtime
        .store()
        .get_profile("profile-bridge")
        .unwrap()
        .unwrap();
    profile.credential_ref = Some("credential-secret-bridge".to_string());
    runtime.store_mut().update_profile(&profile).unwrap();
    let project = crate::permission_policy::Project {
        id: "project-safe".to_string(),
        name: "普通项目".to_string(),
        primary_root: std::env::temp_dir().join("pad-user-project-safe"),
        profile_id: Some(profile.id.clone()),
        ..Default::default()
    };
    runtime.store_mut().insert_project(&project).unwrap();
    let task = Task {
        id: "task-safe".to_string(),
        project_id: Some(project.id.clone()),
        profile_id: profile.id.clone(),
        pi_session_id: Some("pi-private-session-id".to_string()),
        session_file: Some(profile.session_dir.join("private-session.jsonl")),
        title: "普通任务".to_string(),
        summary: "用户可见摘要".to_string(),
        cwd: project.primary_root.clone(),
        ..Default::default()
    };
    runtime.store_mut().insert_task(&task).unwrap();

    let (legacy, _) = handle_line(&mut runtime, r#"{"id":"b1","action":"bootstrap"}"#);
    let legacy_json = serde_json::to_string(&legacy).unwrap();
    assert!(legacy_json.contains("credential_ref"));
    assert!(legacy_json.contains("pi_session_id"));

    let (safe, _) = handle_line(
        &mut runtime,
        r#"{"id":"b2","action":"bootstrap","protocol_version":2}"#,
    );
    assert!(safe.ok);
    let safe_json = serde_json::to_string(&safe).unwrap();
    for forbidden in [
        "agent_dir",
        "session_dir",
        "credential_ref",
        "credential-secret-bridge",
        "pi_session_id",
        "session_file",
        "pi-private-session-id",
    ] {
        assert!(
            !safe_json.contains(forbidden),
            "leaked {forbidden}: {safe_json}"
        );
    }
    assert!(safe_json.contains("普通任务"));
    assert!(safe_json.contains("用户可见摘要"));
    assert!(safe_json.contains("pad-user-project-safe"));
}

pub(crate) fn v2_redaction_covers_every_default_protected_namespace() {
    let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    let default_roots = crate::permission_policy::default_protected_namespaces(&home)
        .into_iter()
        .map(|namespace| namespace.root)
        .collect::<Vec<_>>();
    let ordinary_workspace = std::env::current_dir()
        .unwrap()
        .join("renderer-visible-workspace");
    let custom_codex_home = home.join("custom-codex-home-for-renderer-redaction");
    let profile_agent_dir = home.join("custom-profile-agent-dir");
    let profile_session_dir = home.join("custom-profile-session-dir");
    let profile = Profile {
        agent_dir: profile_agent_dir.clone(),
        session_dir: profile_session_dir.clone(),
        ..Profile::default()
    };
    let collected = protocol::private_roots_with_inputs_for_test(
        &[profile],
        default_roots.last().unwrap().clone(),
        Some(&home),
        Some(custom_codex_home.clone()),
    );
    for root in
        default_roots
            .iter()
            .chain([&custom_codex_home, &profile_agent_dir, &profile_session_dir])
    {
        assert_eq!(
            collected
                .iter()
                .filter(|candidate| *candidate == root)
                .count(),
            1,
            "private root was missing or duplicated: {}",
            root.display()
        );
    }

    // Renderer redaction must not depend on the process-wide HOME remaining
    // stable while parallel tests or a host launcher adjust the environment.
    // Product namespace markers are therefore redacted even when the path was
    // captured under a different home than the current process.
    let foreign_roots = crate::permission_policy::default_protected_namespaces(Path::new(
        "/private/foreign-pad-home",
    ))
    .into_iter()
    .map(|namespace| namespace.root)
    .collect::<Vec<_>>();

    let mut runtime = runtime();
    runtime
        .store_mut()
        .insert_project(&crate::permission_policy::Project {
            id: "project-protected-roots".to_string(),
            name: "Protected roots fixture".to_string(),
            primary_root: ordinary_workspace.clone(),
            additional_roots: default_roots.clone(),
            profile_id: Some("profile-bridge".to_string()),
            ..Default::default()
        })
        .unwrap();
    runtime
        .store_mut()
        .insert_task(&Task {
            id: "task-protected-root".to_string(),
            project_id: Some("project-protected-roots".to_string()),
            profile_id: "profile-bridge".to_string(),
            title: "Protected root fixture".to_string(),
            cwd: default_roots[1].clone(),
            ..Default::default()
        })
        .unwrap();

    // Protocol v1 retains its exact legacy record shape; only v2 applies the
    // renderer-safe DTO contract.
    let (legacy, _) = handle_line(&mut runtime, r#"{"id":"legacy","action":"bootstrap"}"#);
    let legacy_json = serde_json::to_string(&legacy).unwrap();
    assert!(legacy_json.contains(ordinary_workspace.to_string_lossy().as_ref()));
    for root in &default_roots {
        assert!(
            legacy_json.contains(root.to_string_lossy().as_ref()),
            "v1 compatibility lost root {}",
            root.display()
        );
    }

    let (safe, _) = handle_line(
        &mut runtime,
        r#"{"id":"safe","action":"bootstrap","protocol_version":2}"#,
    );
    let safe_json = serde_json::to_string(&safe).unwrap();
    assert!(safe_json.contains(ordinary_workspace.to_string_lossy().as_ref()));
    for root in &default_roots {
        assert!(
            !safe_json.contains(root.to_string_lossy().as_ref()),
            "v2 records leaked {}: {safe_json}",
            root.display()
        );
    }

    let protected_text = default_roots
        .iter()
        .map(|root| root.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    let mut tool_output = json!({
        "messages": [{
            "role": "tool",
            "content": format!("{protected_text} {}", ordinary_workspace.display()),
        }]
    });
    protocol::sanitize_v2_result(&runtime, &mut tool_output).unwrap();
    let tool_json = serde_json::to_string(&tool_output).unwrap();
    assert!(tool_json.contains(ordinary_workspace.to_string_lossy().as_ref()));
    for root in &default_roots {
        assert!(!tool_json.contains(root.to_string_lossy().as_ref()));
    }

    let error = protocol::sanitize_v2_error_message(
        &runtime,
        &format!(
            "failed in {protected_text}; workspace {}",
            ordinary_workspace.display()
        ),
    );
    assert!(error.contains(ordinary_workspace.to_string_lossy().as_ref()));
    for root in &default_roots {
        assert!(!error.contains(root.to_string_lossy().as_ref()));
    }

    let foreign_text = foreign_roots
        .iter()
        .map(|root| root.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    let foreign_error = protocol::sanitize_v2_error_message(&runtime, &foreign_text);
    for root in foreign_roots {
        assert!(
            !foreign_error.contains(root.to_string_lossy().as_ref()),
            "foreign protected namespace leaked after HOME changed: {} in {foreign_error}",
            root.display()
        );
    }
}

#[cfg(unix)]
pub(crate) fn protocol_v2_enforces_active_profile_for_records_and_task_controls() {
    use crate::pad_store::{DesktopTheme, SidebarWidth};
    use crate::permission_policy::Project;
    use std::fs;

    let root = std::env::temp_dir().join(format!(
        "pad-profile-isolation-{}-{}",
        std::process::id(),
        crate::time::unix_now_nanos()
    ));
    let workspace_a = root.join("workspace-a");
    let workspace_b = root.join("workspace-b-secret-marker");
    let sessions_a = root.join("sessions-a");
    let sessions_b = root.join("sessions-b");
    for path in [&workspace_a, &workspace_b, &sessions_a, &sessions_b] {
        fs::create_dir_all(path).unwrap();
    }
    let session_b = sessions_b.join("private-b.jsonl");
    fs::write(
        &session_b,
        "{\"type\":\"message\",\"message\":{\"role\":\"user\",\"content\":\"B-PRIVATE-CONTENT\"}}\n",
    )
    .unwrap();

    let mut runtime = DesktopRuntime::in_memory().unwrap();
    crate::desktop_runtime::tests::configure_fake_pi(
        &mut runtime,
        &[json!({"type": "agent_settled"})],
    );
    runtime.set_terminal_program_for_test(PathBuf::from("/bin/sh"));
    for (id, name, session_dir) in [
        ("profile-a", "Account A", sessions_a.clone()),
        ("profile-b", "Account B", sessions_b.clone()),
    ] {
        runtime
            .store_mut()
            .insert_profile(&Profile {
                id: id.to_string(),
                name: name.to_string(),
                agent_dir: root.join(format!("agent-{id}")),
                session_dir,
                ..Default::default()
            })
            .unwrap();
    }
    for (id, name, root, profile_id) in [
        ("project-a", "Project A", workspace_a.clone(), "profile-a"),
        (
            "project-b-private-id",
            "Project B private title",
            workspace_b.clone(),
            "profile-b",
        ),
    ] {
        runtime
            .store_mut()
            .insert_project(&Project {
                id: id.to_string(),
                name: name.to_string(),
                primary_root: root,
                profile_id: Some(profile_id.to_string()),
                ..Default::default()
            })
            .unwrap();
    }
    for (id, project_id, profile_id, cwd, session_file) in [
        ("task-a", "project-a", "profile-a", workspace_a, None),
        (
            "task-b-private-id",
            "project-b-private-id",
            "profile-b",
            workspace_b,
            Some(session_b),
        ),
    ] {
        runtime
            .store_mut()
            .insert_task(&Task {
                id: id.to_string(),
                project_id: Some(project_id.to_string()),
                profile_id: profile_id.to_string(),
                session_file,
                title: format!("{id}-private-title"),
                summary: format!("{id}-private-summary"),
                cwd,
                ..Default::default()
            })
            .unwrap();
    }
    runtime
        .set_desktop_ui_state(DesktopUiState {
            active_profile_id: Some("profile-a".to_string()),
            selected_task_id: Some("task-a".to_string()),
            collapsed_section_ids: Vec::new(),
            collapsed_project_ids: Vec::new(),
            sidebar_width: SidebarWidth::new(275).unwrap(),
            sidebar_view: crate::pad_store::DesktopSidebarView::All,
            theme: DesktopTheme::System,
            right_panel_open: false,
            bottom_panel_open: false,
            sidebar_open: true,
        })
        .unwrap();

    let (bootstrap, _) = handle_line(
        &mut runtime,
        r#"{"id":"isolation-bootstrap","action":"bootstrap","protocol_version":2}"#,
    );
    assert!(bootstrap.ok, "bootstrap failed: {bootstrap:?}");
    let bootstrap = bootstrap.result.unwrap();
    let records = &bootstrap["records"];
    assert_eq!(records["projects"].as_array().unwrap().len(), 1);
    assert_eq!(records["projects"][0]["id"], "project-a");
    assert_eq!(records["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(records["tasks"][0]["id"], "task-a");
    let sidebar = serde_json::to_string(&bootstrap["sidebar"]).unwrap();
    for forbidden in [
        "project-b-private-id",
        "Project B private title",
        "task-b-private-id",
        "task-b-private-title",
        "workspace-b-secret-marker",
        "B-PRIVATE-CONTENT",
    ] {
        assert!(
            !sidebar.contains(forbidden),
            "active A sidebar leaked {forbidden}: {sidebar}"
        );
    }

    let denied = [
        json!({"action":"history","task_id":"task-b-private-id"}),
        json!({"action":"start_task","task_id":"task-b-private-id"}),
        json!({"action":"retry_task","task_id":"task-b-private-id"}),
        json!({"action":"prompt","task_id":"task-b-private-id","prompt":"should not run"}),
        json!({"action":"poll","task_id":"task-b-private-id"}),
        json!({"action":"abort","task_id":"task-b-private-id"}),
        json!({"action":"stop","task_id":"task-b-private-id"}),
        json!({"action":"get_messages","task_id":"task-b-private-id"}),
        json!({"action":"get_state","task_id":"task-b-private-id"}),
        json!({"action":"get_entries","task_id":"task-b-private-id"}),
        json!({"action":"set_task","task_id":"task-b-private-id","pinned":true}),
        json!({"action":"set_model","task_id":"task-b-private-id","provider":"test","model":"test"}),
        json!({"action":"set_thinking_level","task_id":"task-b-private-id","thinking_level":"high"}),
        json!({"action":"respond_ui","task_id":"task-b-private-id","request_id":"request","value":true}),
        json!({"action":"runtime_snapshot","task_id":"task-b-private-id"}),
        json!({"action":"terminal_open","task_id":"task-b-private-id","pane_id":"pane-b"}),
    ];
    for (index, mut request) in denied.into_iter().enumerate() {
        request["id"] = json!(format!("denied-{index}"));
        request["protocol_version"] = json!(2);
        let action = request["action"].as_str().unwrap().to_string();
        let (response, _) = handle_line(&mut runtime, &request.to_string());
        assert!(!response.ok, "{action} crossed Profile boundary");
        let error = response.error.unwrap();
        assert_eq!(error.code, "task_not_found", "wrong code for {action}");
        assert_eq!(
            error.message, "task is unavailable for the active profile",
            "unsafe error for {action}"
        );
    }

    for request in [
        json!({"id":"project-b-denied","action":"create_project","protocol_version":2,"profile_id":"profile-b","name":"denied","cwd":root}),
        json!({"id":"task-b-create-denied","action":"create_task","protocol_version":2,"profile_id":"profile-b","title":"denied","cwd":root}),
        json!({"id":"profile-b-set-denied","action":"set_profile","protocol_version":2,"profile_id":"profile-b","unattended":true}),
        json!({"id":"profile-b-status-denied","action":"provider_status","protocol_version":2,"profile_id":"profile-b"}),
        json!({"id":"profile-b-auth-denied","action":"auth_begin","protocol_version":2,"profile_id":"profile-b","provider":"test","auth_type":"oauth"}),
        json!({"id":"profile-b-auth-status-denied","action":"auth_status","protocol_version":2,"profile_id":"profile-b"}),
        json!({"id":"profile-b-logout-denied","action":"logout","protocol_version":2,"profile_id":"profile-b","provider":"test"}),
    ] {
        let (response, _) = handle_line(&mut runtime, &request.to_string());
        assert!(
            !response.ok,
            "profile-scoped request unexpectedly succeeded"
        );
        assert_eq!(response.error.unwrap().code, "profile_not_active");
    }

    let switch_b = json!({
        "id": "switch-b",
        "action": "set_ui_state",
        "protocol_version": 2,
        "state": {
            "active_profile_id": "profile-b",
            "selected_task_id": "task-b-private-id",
            "collapsed_section_ids": [],
            "collapsed_project_ids": ["project:project-a"],
            "sidebar_width": 275,
            "theme": "system",
            "right_panel_open": false,
            "bottom_panel_open": false,
            "sidebar_open": true
        }
    });
    let (switched, _) = handle_line(&mut runtime, &switch_b.to_string());
    assert!(switched.ok, "valid account switch failed: {switched:?}");
    assert!(
        switched.result.as_ref().unwrap()["state"]["collapsed_project_ids"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let history_b = json!({
        "id": "history-b",
        "action": "history",
        "protocol_version": 2,
        "task_id": "task-b-private-id"
    });
    let (history_b, _) = handle_line(&mut runtime, &history_b.to_string());
    assert!(history_b.ok, "active B history failed: {history_b:?}");
    assert!(serde_json::to_string(&history_b)
        .unwrap()
        .contains("B-PRIVATE-CONTENT"));

    let open_b = json!({
        "id": "open-b",
        "action": "terminal_open",
        "protocol_version": 2,
        "task_id": "task-b-private-id",
        "pane_id": "pane-b",
        "columns": 80,
        "rows": 24
    });
    let (open_b, _) = handle_line(&mut runtime, &open_b.to_string());
    assert!(open_b.ok, "active B terminal failed: {open_b:?}");

    let switch_a = json!({
        "id": "switch-a",
        "action": "set_ui_state",
        "protocol_version": 2,
        "state": {
            "active_profile_id": "profile-a",
            "selected_task_id": "task-a",
            "collapsed_section_ids": [],
            "collapsed_project_ids": [],
            "sidebar_width": 275,
            "theme": "system",
            "right_panel_open": false,
            "bottom_panel_open": false,
            "sidebar_open": true
        }
    });
    assert!(handle_line(&mut runtime, &switch_a.to_string()).0.ok);
    for request in [
        json!({"id":"pane-input-denied","action":"terminal_input","protocol_version":2,"pane_id":"pane-b","data":"echo denied\n"}),
        json!({"id":"pane-resize-denied","action":"terminal_resize","protocol_version":2,"pane_id":"pane-b","columns":90,"rows":30}),
        json!({"id":"pane-snapshot-denied","action":"terminal_snapshot","protocol_version":2,"pane_id":"pane-b"}),
        json!({"id":"pane-close-denied","action":"terminal_close","protocol_version":2,"pane_id":"pane-b"}),
    ] {
        let (response, _) = handle_line(&mut runtime, &request.to_string());
        assert!(!response.ok, "cross-Profile pane control succeeded");
        let error = response.error.unwrap();
        assert_eq!(error.code, "terminal_pane_not_found");
        assert_eq!(
            error.message,
            "terminal pane is unavailable for the active profile"
        );
    }

    // Pane IDs are namespaced by Profile: using the same renderer-facing ID
    // in A neither reveals nor controls B's still-running pane.
    let open_a_same_id = json!({
        "id": "open-a-same-pane-id",
        "action": "terminal_open",
        "protocol_version": 2,
        "task_id": "task-a",
        "pane_id": "pane-b",
        "columns": 80,
        "rows": 24
    });
    let (open_a_same_id, _) = handle_line(&mut runtime, &open_a_same_id.to_string());
    assert!(
        open_a_same_id.ok,
        "Profile-scoped pane ID was treated as global: {open_a_same_id:?}"
    );
    let close_a_same_id = json!({
        "id": "close-a-same-pane-id",
        "action": "terminal_close",
        "protocol_version": 2,
        "pane_id": "pane-b"
    });
    assert!(handle_line(&mut runtime, &close_a_same_id.to_string()).0.ok);

    assert!(handle_line(&mut runtime, &switch_b.to_string()).0.ok);
    let close_b = json!({
        "id": "close-b",
        "action": "terminal_close",
        "protocol_version": 2,
        "pane_id": "pane-b"
    });
    let (close_b, _) = handle_line(&mut runtime, &close_b.to_string());
    assert!(close_b.ok, "pane owner could not close pane: {close_b:?}");
}

pub(crate) fn v2_history_poll_and_events_redact_private_tool_data_only() {
    let runtime = runtime();
    let ordinary_user = "请解释这段构建日志，普通的 token 概念文字要保留";
    let ordinary_assistant = "构建已经完成，可以继续。";
    for surface in ["history", "get_messages", "get_entries", "poll"] {
        let mut result = json!({
            "surface": surface,
            "messages": [
                {"role": "user", "content": ordinary_user},
                {"role": "assistant", "content": ordinary_assistant},
                {"role": "tool", "content": concat!(
                    "PI_CODING_AGENT_DIR=/Users/tim/Library/Application Support/PAD Desktop/v1/profiles/p/pi-agent ",
                    "PI_CODING_AGENT_SESSION_DIR=/Users/tim/.codex/sessions ",
                    "PI_SESSION_FILE=/private/session.jsonl ",
                    "{\"access_token\":\"token-secret-value\",\"credential_ref\":\"keychain-secret\"}"
                )}
            ]
        });
        protocol::sanitize_v2_result(&runtime, &mut result).unwrap();
        let encoded = serde_json::to_string(&result).unwrap();
        for forbidden in [
            "PI_CODING_AGENT_DIR",
            "PI_CODING_AGENT_SESSION_DIR",
            "PI_SESSION_FILE",
            "/Library/Application Support/PAD Desktop",
            ".codex",
            "token-secret-value",
            "keychain-secret",
        ] {
            assert!(
                !encoded.contains(forbidden),
                "{surface} leaked {forbidden}: {encoded}"
            );
        }
        assert!(encoded.contains(ordinary_user));
        assert!(encoded.contains(ordinary_assistant));

        let event = protocol::event_frame(1, "runtime_changed", result);
        let event_json = serde_json::to_string(&event).unwrap();
        assert!(!event_json.contains("PI_SESSION_FILE"));
        assert!(!event_json.contains("token-secret-value"));
        assert!(event_json.contains(ordinary_user));
    }

    let error = protocol::sanitize_v2_error_message(
        &runtime,
        "failed at /Users/tim/Library/Application Support/PAD Desktop/private and /Users/tim/.codex/state",
    );
    assert!(!error.contains("/Library/Application Support/PAD Desktop"));
    assert!(!error.contains(".codex"));
}

pub(crate) fn actual_v2_history_and_poll_routes_apply_redaction() {
    use std::fs;

    let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    let protected_roots = crate::permission_policy::default_protected_namespaces(&home)
        .into_iter()
        .map(|namespace| namespace.root)
        .collect::<Vec<_>>();
    let protected_text = protected_roots
        .iter()
        .map(|root| root.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    let root = std::env::temp_dir().join(format!(
        "pad-v2-redaction-{}-{}",
        std::process::id(),
        crate::time::unix_now_nanos()
    ));
    let agent_dir = root.join("agent");
    let session_dir = root.join("sessions");
    fs::create_dir_all(&agent_dir).unwrap();
    fs::create_dir_all(&session_dir).unwrap();
    let session_file = session_dir.join("history.jsonl");
    let history_fixture = format!(
        "{}\n{}\n",
        json!({"type":"message","message":{"role":"user","content":"普通历史文本"}}),
        json!({"type":"message","message":{"role":"tool","content":format!("PI_SESSION_FILE=/private/session.jsonl {protected_text}")}}),
    );
    fs::write(&session_file, history_fixture).unwrap();
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    crate::desktop_runtime::tests::configure_fake_pi(
        &mut runtime,
        &[json!({
            "type": "tool_execution_start",
            "toolCallId": "tool-1",
            "content": format!("PI_CODING_AGENT_DIR=/private/agent OPENAI_API_KEY=token-secret-live {protected_text}"),
            "access_token": "token-secret-live",
        })],
    );
    runtime
        .store_mut()
        .insert_profile(&Profile {
            id: "profile-redaction".to_string(),
            name: "Redaction".to_string(),
            agent_dir,
            session_dir,
            ..Default::default()
        })
        .unwrap();
    runtime
        .store_mut()
        .insert_task(&Task {
            id: "task-redaction".to_string(),
            profile_id: "profile-redaction".to_string(),
            session_file: Some(session_file),
            title: "Redaction".to_string(),
            cwd: std::env::temp_dir(),
            ..Default::default()
        })
        .unwrap();

    let history = DesktopRequest {
        action: Some("history".to_string()),
        protocol_version: Some(2),
        task_id: Some("task-redaction".to_string()),
        ..Default::default()
    };
    let history = handle_request(&mut runtime, &history).unwrap();
    let history_json = serde_json::to_string(&history).unwrap();
    assert!(history_json.contains("普通历史文本"));
    assert!(!history_json.contains("PI_SESSION_FILE"));
    for root in &protected_roots {
        assert!(
            !history_json.contains(root.to_string_lossy().as_ref()),
            "history leaked {}: {history_json}",
            root.display()
        );
    }

    let start = DesktopRequest {
        action: Some("start_task".to_string()),
        protocol_version: Some(2),
        task_id: Some("task-redaction".to_string()),
        ..Default::default()
    };
    handle_request(&mut runtime, &start).unwrap();
    let poll = DesktopRequest {
        action: Some("poll".to_string()),
        protocol_version: Some(2),
        task_id: Some("task-redaction".to_string()),
        ..Default::default()
    };
    let mut poll_json = String::new();
    for _ in 0..100 {
        poll_json = serde_json::to_string(&handle_request(&mut runtime, &poll).unwrap()).unwrap();
        if poll_json.contains("tool-1") {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(poll_json.contains("tool-1"));
    for forbidden in ["PI_CODING_AGENT_DIR", "access_token", "token-secret-live"] {
        assert!(
            !poll_json.contains(forbidden),
            "poll leaked {forbidden}: {poll_json}"
        );
    }
    for root in &protected_roots {
        assert!(
            !poll_json.contains(root.to_string_lossy().as_ref()),
            "poll leaked {}: {poll_json}",
            root.display()
        );
    }
}

pub(crate) fn actual_v2_error_route_redacts_private_session_path() {
    let mut runtime = runtime();
    runtime
        .store_mut()
        .insert_task(&Task {
            id: "task-private-error".to_string(),
            profile_id: "profile-bridge".to_string(),
            session_file: Some(PathBuf::from(
                "/Users/tim/Library/Application Support/PAD Desktop/v1/profiles/private/.codex/session.jsonl",
            )),
            title: "Safe visible title".to_string(),
            cwd: std::env::temp_dir(),
            ..Default::default()
        })
        .unwrap();
    let request = json!({
        "id": "private-error",
        "action": "history",
        "protocol_version": 2,
        "task_id": "task-private-error",
    });
    let (response, _) = handle_line(&mut runtime, &request.to_string());
    assert!(!response.ok);
    let encoded = serde_json::to_string(&response).unwrap();
    assert!(encoded.contains("invalid_session_path"));
    assert!(!encoded.contains("/Library/Application Support/PAD Desktop"));
    assert!(!encoded.contains(".codex"));
    assert!(!encoded.contains("session.jsonl"));
}
