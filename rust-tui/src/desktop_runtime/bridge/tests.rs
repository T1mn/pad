use super::*;
use crate::pi_runtime::PiPoll;
use std::path::Path;
use std::thread;
use std::time::Duration;

fn runtime() -> DesktopRuntime {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile = Profile {
        id: "profile-bridge".to_string(),
        name: "Bridge profile".to_string(),
        agent_dir: std::env::temp_dir().join("pad-bridge-agent"),
        session_dir: std::env::temp_dir().join("pad-bridge-sessions"),
        ..Profile::default()
    };
    runtime.store_mut().insert_profile(&profile).unwrap();
    runtime
}

pub(crate) fn protocol_returns_codex_sidebar_for_bootstrap() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let request: DesktopRequest =
        serde_json::from_str(r#"{"id":"b1","action":"bootstrap"}"#).unwrap();
    let result = handle_request(&mut runtime, &request).unwrap();
    assert_eq!(result["protocol_version"], DESKTOP_PROTOCOL_VERSION);
    assert_eq!(result["sidebar"]["view"], "all");
    assert_eq!(result["profile"]["id"], "default");
    assert_eq!(result["backend"]["status"], "ready");
    assert!(result["backend"]["authenticated_providers"].is_array());
}

pub(crate) fn create_task_start_poll_and_stop_round_trip() {
    let mut runtime = runtime();
    let task_request: DesktopRequest = serde_json::from_value(json!({
        "action": "create_task",
        "profile_id": "profile-bridge",
        "task_id": "task-bridge",
        "title": "Bridge task",
        "cwd": std::env::temp_dir(),
    }))
    .unwrap();
    let result = handle_request(&mut runtime, &task_request).unwrap();
    assert_eq!(result["task"]["id"], "task-bridge");

    let start_request: DesktopRequest = serde_json::from_value(json!({
        "action": "start_task",
        "task_id": "task-bridge",
        "command": "/bin/sh -c 'printf \"%s\\n\" \"{\\\"type\\\":\\\"agent_settled\\\"}\"'",
    }))
    .unwrap();
    handle_request(&mut runtime, &start_request).unwrap();
    let poll_request: DesktopRequest = serde_json::from_value(json!({
        "action": "poll",
        "task_id": "task-bridge",
    }))
    .unwrap();
    let mut result = handle_request(&mut runtime, &poll_request).unwrap();
    for _ in 0..30 {
        if result["poll"]["events"]
            .as_array()
            .is_some_and(|events| !events.is_empty())
        {
            break;
        }
        thread::sleep(Duration::from_millis(5));
        result = handle_request(&mut runtime, &poll_request).unwrap();
    }
    assert!(result["poll"]["events"]
        .as_array()
        .is_some_and(|events| !events.is_empty()));
    let stop_request: DesktopRequest = serde_json::from_value(json!({
        "action": "stop_task",
        "task_id": "task-bridge",
    }))
    .unwrap();
    assert_eq!(
        handle_request(&mut runtime, &stop_request).unwrap()["stopped"],
        true
    );
    assert!(!runtime.is_running("task-bridge"));
    assert!(Path::new(".").exists());
}

pub(crate) fn create_project_persists_the_selected_workspace_root() {
    let mut runtime = runtime();
    let root = std::env::temp_dir().join("pad-selected-project");
    let request: DesktopRequest = serde_json::from_value(json!({
        "action": "create_project",
        "profile_id": "profile-bridge",
        "name": "Selected project",
        "cwd": root,
    }))
    .unwrap();
    let result = handle_request(&mut runtime, &request).unwrap();
    assert_eq!(result["project"]["name"], "Selected project");
    assert_eq!(
        result["project"]["primary_root"],
        root.to_string_lossy().as_ref()
    );
    assert_eq!(result["project"]["profile_id"], "profile-bridge");
}

pub(crate) fn malformed_input_is_one_error_response_and_does_not_write_stdout() {
    let mut runtime = runtime();
    let (response, stop) = handle_line(&mut runtime, "not json");
    assert!(!stop);
    assert!(!response.ok);
    assert_eq!(response.error.as_ref().unwrap().code, "invalid_json");
}

pub(crate) fn full_access_fields_are_persisted_on_profile_and_task_requests() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile_request: DesktopRequest = serde_json::from_value(json!({
        "action": "create_profile",
        "profile_id": "full",
        "name": "Full",
        "permission_mode": "system_full",
        "unattended": true,
    }))
    .unwrap();
    let profile_result = handle_request(&mut runtime, &profile_request).unwrap();
    assert_eq!(profile_result["profile"]["policy"]["mode"], "system_full");
    assert_eq!(profile_result["profile"]["policy"]["unattended"], true);
}

pub(crate) fn set_task_persists_sidebar_flags() {
    let mut runtime = runtime();
    let create_request: DesktopRequest = serde_json::from_value(json!({
        "action": "create_task",
        "profile_id": "profile-bridge",
        "task_id": "task-flags",
        "title": "Flagged task",
        "cwd": std::env::temp_dir(),
    }))
    .unwrap();
    handle_request(&mut runtime, &create_request).unwrap();

    let set_request: DesktopRequest = serde_json::from_value(json!({
        "action": "set_task",
        "task_id": "task-flags",
        "pinned": true,
        "archived": true,
    }))
    .unwrap();
    let result = handle_request(&mut runtime, &set_request).unwrap();
    assert_eq!(result["task"]["pinned"], true);
    assert_eq!(result["task"]["archived"], true);
    let stored = runtime.store().get_task("task-flags").unwrap().unwrap();
    assert!(stored.pinned);
    assert!(stored.archived);
}

pub(crate) fn set_profile_persists_policy() {
    let mut runtime = runtime();
    let request: DesktopRequest = serde_json::from_value(json!({
        "action": "set_profile",
        "profile_id": "profile-bridge",
        "permission_mode": "guarded",
        "unattended": false,
    }))
    .unwrap();

    let result = handle_request(&mut runtime, &request).unwrap();
    assert_eq!(result["profile"]["policy"]["mode"], "guarded");
    assert_eq!(result["profile"]["policy"]["unattended"], false);
    let stored = runtime
        .store()
        .get_profile("profile-bridge")
        .unwrap()
        .unwrap();
    assert_eq!(stored.policy.mode, Some(PermissionMode::Guarded));
    assert_eq!(stored.policy.unattended, Some(false));
}

pub(crate) fn response_shape_is_id_ok_result_or_error() {
    let mut runtime = runtime();
    let (response, _) = handle_line(&mut runtime, r#"{"id":"x","action":"ping"}"#);
    let value = serde_json::to_value(response).unwrap();
    assert_eq!(value["id"], "x");
    assert_eq!(value["ok"], true);
    assert!(value.get("result").is_some());
    assert!(value.get("error").is_none());
}

pub(crate) fn renderer_alias_fields_deserialize_for_model_and_ui_response() {
    let model: DesktopRequest = serde_json::from_value(json!({
        "action": "set_model",
        "task_id": "task",
        "provider": "anthropic",
        "model": "claude-sonnet",
    }))
    .unwrap();
    assert_eq!(model.provider.as_deref(), Some("anthropic"));
    assert_eq!(model.model.as_deref(), Some("claude-sonnet"));

    let response: DesktopRequest = serde_json::from_value(json!({
        "action": "extension_ui_response",
        "task_id": "task",
        "interaction_id": "request-1",
        "response_kind": "confirm",
        "value": true,
    }))
    .unwrap();
    assert_eq!(response.interaction_id.as_deref(), Some("request-1"));
    assert_eq!(response.response_kind.as_deref(), Some("confirm"));
    assert_eq!(response.value, Some(Value::Bool(true)));
}

pub(crate) fn provider_status_reports_profile_scoped_authentication_shape() {
    let mut runtime = runtime();
    let request: DesktopRequest = serde_json::from_value(json!({
        "action": "provider_status",
        "profile_id": "profile-bridge",
    }))
    .unwrap();
    let result = handle_request(&mut runtime, &request).unwrap();
    assert_eq!(result["profile_id"], "profile-bridge");
    assert_eq!(result["status"], "ready");
    assert!(result["authenticated_providers"].is_array());
    assert!(result.get("provider_authentication").is_some());
}

pub(crate) fn poll_exposes_structured_ui_requests_without_answering_them() {
    let poll = PiPoll {
        messages: vec![crate::pi_runtime::PiMessage {
            message_type: "extension_ui_request".to_string(),
            id: Some("select-1".to_string()),
            value: json!({
                "type": "extension_ui_request",
                "method": "select",
                "id": "select-1",
                "title": "Choose deployment target",
                "options": ["staging", "production"],
                "defaultIndex": 0,
            }),
        }],
        ..PiPoll::default()
    };
    let value = poll_value(&poll);
    assert_eq!(value["pending_ui_requests"][0]["kind"], "select");
    assert_eq!(
        value["pending_ui_requests"][0]["response_action"],
        "respond_ui"
    );
    assert_eq!(value["pending_ui_requests"][0]["requires_response"], true);
}

pub(crate) fn provider_auth_errors_are_distinguished_from_transport_errors() {
    let auth_error = PiPoll {
        messages: vec![crate::pi_runtime::PiMessage {
            message_type: "response".to_string(),
            id: None,
            value: json!({
                "type": "response",
                "command": "prompt",
                "success": false,
                "error": "No API key found for selected model"
            }),
        }],
        ..PiPoll::default()
    };
    assert!(poll_has_provider_auth_error(&auth_error));
    let transport_error = PiPoll {
        messages: vec![crate::pi_runtime::PiMessage {
            message_type: "response".to_string(),
            id: None,
            value: json!({
                "type": "response",
                "command": "get_state",
                "success": false,
                "error": "unknown command"
            }),
        }],
        ..PiPoll::default()
    };
    assert!(!poll_has_provider_auth_error(&transport_error));
}
