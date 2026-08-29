use super::*;
use crate::permission_policy::{PolicyLayer, Project};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

fn profile() -> Profile {
    Profile {
        id: "profile-runtime".to_string(),
        name: "Runtime Profile".to_string(),
        agent_dir: std::env::temp_dir().join("pad-desktop-runtime-agent"),
        session_dir: std::env::temp_dir().join("pad-desktop-runtime-sessions"),
        ..Default::default()
    }
}

fn task() -> Task {
    Task {
        id: "task-runtime".to_string(),
        profile_id: "profile-runtime".to_string(),
        cwd: std::env::temp_dir(),
        title: "Runtime task".to_string(),
        ..Default::default()
    }
}

pub(crate) fn profile_scoped_process_events_update_the_private_task_record() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile = profile();
    let project = Project {
        id: "project-runtime".to_string(),
        name: "Runtime project".to_string(),
        primary_root: PathBuf::from("/tmp"),
        profile_id: Some(profile.id.clone()),
        policy: PolicyLayer::default(),
        ..Default::default()
    };
    runtime.store_mut().insert_profile(&profile).unwrap();
    runtime.store_mut().insert_project(&project).unwrap();
    let mut stored_task = task();
    stored_task.project_id = Some(project.id.clone());
    runtime.store_mut().insert_task(&stored_task).unwrap();

    runtime
        .start_task("task-runtime", "/bin/echo '{\"type\":\"agent_settled\"}'")
        .unwrap();
    for _ in 0..20 {
        let _ = runtime.poll_task("task-runtime").unwrap();
        if runtime
            .store()
            .get_task("task-runtime")
            .unwrap()
            .unwrap()
            .status
            == TaskStatus::Idle
        {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        runtime
            .store()
            .get_task("task-runtime")
            .unwrap()
            .unwrap()
            .status,
        TaskStatus::Idle
    );
    assert!(!runtime.is_running("missing"));
    runtime.stop_task("task-runtime").unwrap();
    assert!(!runtime.is_running("task-runtime"));
}

pub(crate) fn sidebar_snapshot_is_read_from_the_pad_store_only() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    runtime.store_mut().insert_profile(&profile()).unwrap();
    let snapshot = runtime.sidebar_snapshot().unwrap();
    assert_eq!(
        snapshot.active_profile_id.as_deref(),
        Some("profile-runtime")
    );
    assert!(
        snapshot
            .rows
            .iter()
            .any(|row| row.node
                == crate::sidebar::CodexSidebarNode::Profile("profile-runtime".into()))
    );
}

pub(crate) fn empty_task_cwd_inherits_its_selected_project_root() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile = profile();
    let project_root = std::env::temp_dir().join("pad-selected-project-root");
    let project = Project {
        id: "project-selected".to_string(),
        name: "Selected project".to_string(),
        primary_root: project_root.clone(),
        profile_id: Some(profile.id.clone()),
        ..Project::default()
    };
    runtime.store_mut().insert_profile(&profile).unwrap();
    runtime.store_mut().insert_project(&project).unwrap();

    let mut draft = task();
    draft.cwd = PathBuf::new();
    draft.project_id = Some(project.id);
    let created = runtime.create_task(draft).unwrap();

    assert_eq!(created.cwd, project_root);
}

pub(crate) fn explicit_permission_gate_is_the_only_full_access_ui_auto_response() {
    let permission = automatic_ui_response(&serde_json::json!({
        "type": "extension_ui_request",
        "method": "confirm",
        "id": "allow-1",
        "title": "Allow tool execution",
        "message": "Permit this command to run?"
    }));
    assert_eq!(
        permission.as_ref().and_then(|value| value.get("id")),
        Some(&serde_json::Value::String("allow-1".to_string()))
    );
    assert_eq!(
        permission.as_ref().and_then(|value| value.get("confirmed")),
        Some(&serde_json::Value::Bool(true))
    );
    assert!(permission
        .as_ref()
        .and_then(|value| value.get("value"))
        .is_none());

    assert!(automatic_ui_response(&serde_json::json!({
        "method": "confirm", "id": "allow-zh", "title": "允许执行命令？"
    }))
    .is_some());
    assert!(automatic_ui_response(&serde_json::json!({
        "method": "confirm", "id": "trust-zh", "title": "是否信任此项目？"
    }))
    .is_none());
    assert!(automatic_ui_response(&serde_json::json!({
        "method": "confirm", "id": "protected", "title": "Allow permission?",
        "path": "/Users/example/.codex/auth.json"
    }))
    .is_none());

    for request in [
        serde_json::json!({"method":"confirm", "id":"business", "title":"Continue?"}),
        serde_json::json!({"method":"select", "id":"select", "options":["a","b"], "defaultIndex":1}),
        serde_json::json!({"method":"input", "id":"input", "default":"yes"}),
        serde_json::json!({"method":"editor", "id":"editor", "default":"text"}),
    ] {
        assert!(
            automatic_ui_response(&request).is_none(),
            "request was auto-answered: {request}"
        );
    }
}

pub(crate) fn existing_task_session_is_restored_and_state_metadata_is_persisted() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile = profile();
    std::fs::create_dir_all(&profile.session_dir).unwrap();
    runtime.store_mut().insert_profile(&profile).unwrap();
    let session_file = profile.session_dir.join("restored.jsonl");
    let mut stored_task = task();
    stored_task.session_file = Some(session_file.clone());
    runtime.store_mut().insert_task(&stored_task).unwrap();

    let response = serde_json::json!({
        "type": "response", "command": "get_state", "success": true,
        "data": {"sessionFile": session_file, "sessionId": "restored-session"}
    });
    let command = format!(
        "/bin/echo {}",
        crate::shell_quote::single_quote(&serde_json::to_string(&response).unwrap())
    );
    runtime.start_task("task-runtime", &command).unwrap();
    for _ in 0..20 {
        let _ = runtime.poll_task("task-runtime").unwrap();
        let current = runtime.store().get_task("task-runtime").unwrap().unwrap();
        if current.pi_session_id.as_deref() == Some("restored-session") {
            assert_eq!(current.session_file, stored_task.session_file);
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("get_state response did not persist Pi session metadata");
}

pub(crate) fn existing_task_session_outside_profile_root_is_rejected() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile = profile();
    runtime.store_mut().insert_profile(&profile).unwrap();
    let mut stored_task = task();
    stored_task.session_file = Some(std::env::temp_dir().join("not-profile-session.jsonl"));
    runtime.store_mut().insert_task(&stored_task).unwrap();
    let error = runtime
        .start_task("task-runtime", "/bin/echo '{\"type\":\"agent_settled\"}'")
        .unwrap_err();
    assert!(matches!(
        error,
        DesktopRuntimeError::InvalidSessionPath { .. }
    ));
}

pub(crate) fn history_falls_back_to_read_only_profile_session_journal() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile = profile();
    std::fs::create_dir_all(&profile.session_dir).unwrap();
    runtime.store_mut().insert_profile(&profile).unwrap();
    let session_file = profile.session_dir.join("history.jsonl");
    let journal = [
        serde_json::json!({"type":"session", "id":"history-session"}),
        serde_json::json!({"type":"message", "id":"m1", "message":{"role":"user", "content":"hello"}}),
        serde_json::json!({"type":"message", "id":"m2", "message":{"role":"assistant", "content":"world"}}),
    ]
    .into_iter()
    .map(|entry| serde_json::to_string(&entry).unwrap())
    .collect::<Vec<_>>()
    .join("\n");
    std::fs::write(&session_file, format!("{journal}\n")).unwrap();
    let mut stored_task = task();
    stored_task.session_file = Some(session_file);
    runtime.store_mut().insert_task(&stored_task).unwrap();

    let response = runtime.history("task-runtime").unwrap();
    assert_eq!(response["success"], true);
    assert_eq!(response["data"]["messages"].as_array().unwrap().len(), 2);
    assert_eq!(response["data"]["messages"][0]["content"], "hello");
}
