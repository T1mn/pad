use super::*;
use crate::permission_policy::{PolicyLayer, Project};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

pub(crate) fn configure_fake_pi(runtime: &mut DesktopRuntime, messages: &[serde_json::Value]) {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let root = std::env::temp_dir().join(format!(
        "pad-fake-pi-{}-{}",
        std::process::id(),
        crate::time::unix_now_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let program = root.join("pi");
    let mut script = String::from("#!/bin/sh\nset -eu\n");
    for message in messages {
        let line = serde_json::to_string(message).unwrap();
        script.push_str("printf '%s\\n' ");
        script.push_str(&crate::shell_quote::single_quote(&line));
        script.push('\n');
    }
    script.push_str("while IFS= read -r _pad_line; do :; done\n");
    fs::write(&program, script).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
    runtime.set_pi_program_for_test(program);
}

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
    configure_fake_pi(
        &mut runtime,
        &[serde_json::json!({"type": "agent_settled"})],
    );
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

    runtime.start_task("task-runtime").unwrap();
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

pub(crate) fn empty_owner_pump_does_not_rewrite_task_metadata() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    configure_fake_pi(&mut runtime, &[]);
    runtime.store_mut().insert_profile(&profile()).unwrap();
    runtime.store_mut().insert_task(&task()).unwrap();
    runtime.start_task("task-runtime").unwrap();
    let mut record = runtime.store().get_task("task-runtime").unwrap().unwrap();
    record.updated_at = 17;
    runtime.store_mut().update_task(&record).unwrap();
    for _ in 0..4 {
        assert!(runtime.poll_task("task-runtime").unwrap().is_empty());
    }
    assert_eq!(
        runtime
            .store()
            .get_task("task-runtime")
            .unwrap()
            .unwrap()
            .updated_at,
        17
    );
    runtime.stop_task("task-runtime").unwrap();
}

#[cfg(unix)]
pub(crate) fn existing_task_session_is_passed_to_native_pi_at_startup() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let root = crate::test_support::temp_path("pad-desktop", "native-session-resume");
    let session_dir = root.join("sessions with spaces");
    let agent_dir = root.join("agent");
    fs::create_dir_all(&session_dir).unwrap();
    let session_file = session_dir.join("existing session.jsonl");
    fs::write(&session_file, "{\"type\":\"session\",\"version\":3}\n").unwrap();
    let arguments_file = root.join("arguments.txt");
    let program = root.join("pi");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" > {}\nwhile IFS= read -r _pad_line; do :; done\n",
        crate::shell_quote::single_quote(&arguments_file.to_string_lossy())
    );
    fs::write(&program, script).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();

    let mut runtime = DesktopRuntime::in_memory().unwrap();
    runtime.set_pi_program_for_test(program);
    let mut stored_profile = profile();
    stored_profile.agent_dir = agent_dir;
    stored_profile.session_dir = session_dir;
    let mut stored_task = task();
    stored_task.session_file = Some(session_file.clone());
    runtime.store_mut().insert_profile(&stored_profile).unwrap();
    runtime.store_mut().insert_task(&stored_task).unwrap();

    runtime.start_task(&stored_task.id).unwrap();
    for _ in 0..40 {
        if arguments_file.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let arguments = fs::read_to_string(&arguments_file).unwrap();
    let arguments = arguments.lines().collect::<Vec<_>>();
    assert_eq!(&arguments[..2], &["--mode", "rpc"]);
    assert_eq!(arguments[2], "--extension");
    assert!(arguments[3].ends_with("/pad-fast-mode.ts"));
    assert_eq!(
        &arguments[4..],
        &["--session", session_file.to_str().unwrap()]
    );
    runtime.stop_task(&stored_task.id).unwrap();
    let _ = fs::remove_dir_all(root);
}

pub(crate) fn request_pi_defers_same_batch_interaction_for_single_owner_fanout() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    configure_fake_pi(
        &mut runtime,
        &[
            serde_json::json!({
                "type":"response",
                "command":"get_messages",
                "success":true,
                "data":{"messages":[]}
            }),
            serde_json::json!({
                "type":"extension_ui_request",
                "generation":1,
                "sequence":1,
                "id":"approval-1",
                "method":"confirm",
                "title":"Continue?"
            }),
        ],
    );
    runtime.store_mut().insert_profile(&profile()).unwrap();
    runtime.store_mut().insert_task(&task()).unwrap();
    runtime.start_task("task-runtime").unwrap();
    let history = runtime.history("task-runtime").unwrap();
    assert_eq!(history["command"], "get_messages");
    let deferred = runtime.poll_task("task-runtime").unwrap();
    assert_eq!(
        deferred
            .messages
            .iter()
            .filter(|message| message.message_type == "extension_ui_request")
            .count(),
        1
    );
    assert_eq!(deferred.events.len(), 1);
    assert!(runtime.poll_task("task-runtime").unwrap().is_empty());
    runtime.stop_task("task-runtime").unwrap();
}

pub(crate) fn rpc_responses_do_not_mark_the_task_failed() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    configure_fake_pi(
        &mut runtime,
        &[
            serde_json::json!({
                "type":"response", "command":"get_state", "success":true,
                "data":{"sessionId":"session-rpc"}
            }),
            serde_json::json!({
                "type":"response", "command":"get_messages", "success":true,
                "data":{"messages":[]}
            }),
        ],
    );
    runtime.store_mut().insert_profile(&profile()).unwrap();
    runtime.store_mut().insert_task(&task()).unwrap();
    runtime.start_task("task-runtime").unwrap();
    let history = runtime.history("task-runtime").unwrap();
    assert_eq!(history["command"], "get_messages");
    assert_eq!(
        runtime
            .store()
            .get_task("task-runtime")
            .unwrap()
            .unwrap()
            .status,
        TaskStatus::Starting
    );
    let deferred = runtime.poll_task("task-runtime").unwrap();
    assert!(deferred.events.is_empty());
    runtime.stop_task("task-runtime").unwrap();
}

pub(crate) fn auto_answered_permission_is_not_republished_as_pending_ui() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let work = crate::test_support::temp_path("pad-desktop", "auto-answer-work");
    std::fs::create_dir_all(&work).unwrap();
    configure_fake_pi(
        &mut runtime,
        &[
            serde_json::json!({"type":"agent_start","generation":1,"sequence":1}),
            serde_json::json!({
                "type":"extension_ui_request", "generation":1, "sequence":2,
                "method":"confirm", "id":"permission-1",
                "title":"Allow permission?", "message":"Permit this file edit?",
                "toolName":"edit", "path":work.join("file.txt")
            }),
        ],
    );
    let mut allowed_profile = profile();
    allowed_profile.policy = PolicyLayer {
        mode: Some(PermissionMode::WorkspaceFull),
        unattended: Some(true),
        ..PolicyLayer::default()
    };
    let mut allowed_task = task();
    allowed_task.cwd = work.clone();
    runtime
        .store_mut()
        .insert_profile(&allowed_profile)
        .unwrap();
    runtime.store_mut().insert_task(&allowed_task).unwrap();
    runtime.start_task("task-runtime").unwrap();
    let mut saw_runtime_event = false;
    for _ in 0..20 {
        let poll = runtime.poll_task("task-runtime").unwrap();
        assert!(!poll.messages.iter().any(|message| {
            message.message_type == "extension_ui_request" && message.value["id"] == "permission-1"
        }));
        assert!(!poll
            .events
            .iter()
            .any(|event| event.value["id"] == "permission-1"));
        saw_runtime_event |= poll
            .messages
            .iter()
            .any(|message| message.message_type == "agent_start");
        if saw_runtime_event {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(saw_runtime_event);
    assert_eq!(
        runtime
            .store()
            .get_task("task-runtime")
            .unwrap()
            .unwrap()
            .status,
        TaskStatus::Running
    );
    runtime.stop_task("task-runtime").unwrap();
    let _ = std::fs::remove_dir_all(work);
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
    let profile = Profile {
        id: "approval-profile".into(),
        agent_dir: "/private/profile/pi-agent".into(),
        session_dir: "/private/profile/pi-sessions".into(),
        policy: PolicyLayer {
            mode: Some(PermissionMode::WorkspaceFull),
            unattended: Some(true),
            ..PolicyLayer::default()
        },
        ..Profile::default()
    };
    let project = Project {
        id: "approval-project".into(),
        profile_id: Some(profile.id.clone()),
        primary_root: "/work/project".into(),
        ..Project::default()
    };
    let approval_task = Task {
        id: "approval-task".into(),
        profile_id: profile.id.clone(),
        project_id: Some(project.id.clone()),
        cwd: "/work/project".into(),
        ..Task::default()
    };
    let mandatory =
        crate::permission_policy::default_protected_namespaces(Path::new("/Users/example"));
    let workspace_policy = merge_profile_project_task_with_host_defaults(
        &profile,
        Some(&project),
        Some(&approval_task),
        &mandatory,
    );
    let workspace_request = serde_json::json!({
        "type": "extension_ui_request",
        "method": "confirm",
        "id": "allow-1",
        "title": "Allow tool execution",
        "message": "Permit this command to run?",
        "toolName": "edit",
        "path": "/work/project/src/main.rs"
    });
    let permission = automatic_ui_response(
        &workspace_request,
        &workspace_policy,
        Path::new("/work/project"),
    );
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

    let outside_request = serde_json::json!({
        "method": "confirm", "id": "outside", "title": "Allow permission?",
        "toolName": "write", "path": "/tmp/outside.txt"
    });
    assert!(automatic_ui_response(
        &outside_request,
        &workspace_policy,
        Path::new("/work/project")
    )
    .is_none());

    let mut system_profile = profile.clone();
    system_profile.policy.mode = Some(PermissionMode::SystemFull);
    let system_policy = merge_profile_project_task_with_host_defaults(
        &system_profile,
        Some(&project),
        Some(&approval_task),
        &mandatory,
    );
    assert!(
        automatic_ui_response(&outside_request, &system_policy, Path::new("/work/project"))
            .is_some()
    );

    for protected_path in [
        "/private/profile/pi-agent/auth.json",
        "/private/profile/pi-sessions/session.jsonl",
        "/Users/example/.codex/auth.json",
        "/Users/example/Library/Containers/com.openai.chat/Data/Library/state.sqlite",
        "/Users/example/Library/Application Support/PAD Desktop/v1/store/pad.sqlite",
    ] {
        let request = serde_json::json!({
            "method": "confirm", "id": "protected", "title": "Allow permission?",
            "toolName": "write", "path": protected_path
        });
        assert!(
            automatic_ui_response(&request, &system_policy, Path::new("/work/project")).is_none(),
            "protected path was auto-approved: {protected_path}"
        );
    }
    let credential = serde_json::json!({
        "method": "confirm", "id": "credential", "title": "Allow credential access?",
        "toolName": "credential_store"
    });
    assert!(
        automatic_ui_response(&credential, &system_policy, Path::new("/work/project")).is_none()
    );
    let protected_command = serde_json::json!({
        "method": "confirm", "id": "shell-protected", "title": "Allow command execution?",
        "toolName": "bash", "command": "rm -f ~/.codex/auth.json"
    });
    assert!(automatic_ui_response(
        &protected_command,
        &system_policy,
        Path::new("/work/project")
    )
    .is_none());

    let mut guarded_policy = workspace_policy.clone();
    guarded_policy.mode = PermissionMode::Guarded;
    assert!(automatic_ui_response(
        &workspace_request,
        &guarded_policy,
        Path::new("/work/project")
    )
    .is_none());
    let mut attended_system = system_policy.clone();
    attended_system.unattended = false;
    assert!(automatic_ui_response(
        &outside_request,
        &attended_system,
        Path::new("/work/project")
    )
    .is_none());

    for request in [
        serde_json::json!({"method":"confirm", "id":"business", "title":"Continue?"}),
        serde_json::json!({"method":"select", "id":"select", "options":["a","b"], "defaultIndex":1}),
        serde_json::json!({"method":"input", "id":"input", "default":"yes"}),
        serde_json::json!({"method":"editor", "id":"editor", "default":"text"}),
    ] {
        assert!(
            automatic_ui_response(&request, &system_policy, Path::new("/work/project")).is_none(),
            "request was auto-answered: {request}"
        );
    }
}

pub(crate) fn cross_profile_project_cannot_supply_automatic_approval_policy() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile_a = Profile {
        id: "profile-a".into(),
        name: "A".into(),
        policy: PolicyLayer {
            mode: Some(PermissionMode::SystemFull),
            unattended: Some(true),
            ..PolicyLayer::default()
        },
        ..profile()
    };
    let profile_b = Profile {
        id: "profile-b".into(),
        name: "B".into(),
        ..profile()
    };
    runtime.store_mut().insert_profile(&profile_a).unwrap();
    runtime.store_mut().insert_profile(&profile_b).unwrap();
    let project = Project {
        id: "project-b".into(),
        name: "B project".into(),
        profile_id: Some(profile_b.id.clone()),
        primary_root: "/work/b".into(),
        ..Project::default()
    };
    runtime.store_mut().insert_project(&project).unwrap();
    let mismatched = Task {
        id: "cross-profile".into(),
        profile_id: profile_a.id,
        project_id: Some(project.id),
        cwd: "/work/b".into(),
        ..Task::default()
    };
    runtime.store_mut().insert_task(&mismatched).unwrap();

    assert!(runtime
        .task_policy_context("cross-profile")
        .unwrap()
        .is_none());
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
    configure_fake_pi(&mut runtime, &[response]);
    runtime.start_task("task-runtime").unwrap();
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

pub(crate) fn stale_get_state_cannot_replace_an_existing_task_session() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile = profile();
    std::fs::create_dir_all(&profile.session_dir).unwrap();
    runtime.store_mut().insert_profile(&profile).unwrap();
    let session_file = profile.session_dir.join("durable.jsonl");
    let stale_session_file = profile.session_dir.join("stale-new.jsonl");
    std::fs::write(&session_file, "{\"type\":\"session\",\"version\":3}\n").unwrap();
    let mut stored_task = task();
    stored_task.session_file = Some(session_file.clone());
    stored_task.pi_session_id = Some("durable-session".to_string());
    runtime.store_mut().insert_task(&stored_task).unwrap();

    configure_fake_pi(
        &mut runtime,
        &[serde_json::json!({
            "type": "response", "command": "get_state", "success": true,
            "data": {"sessionFile": stale_session_file, "sessionId": "stale-session"}
        })],
    );
    runtime.start_task("task-runtime").unwrap();
    for _ in 0..20 {
        let _ = runtime.poll_task("task-runtime").unwrap();
        thread::sleep(Duration::from_millis(5));
    }
    let current = runtime.store().get_task("task-runtime").unwrap().unwrap();
    assert_eq!(
        current.session_file.as_deref(),
        Some(session_file.as_path())
    );
    assert_eq!(current.pi_session_id.as_deref(), Some("durable-session"));
    runtime.stop_task("task-runtime").unwrap();
}

pub(crate) fn existing_task_session_outside_profile_root_is_rejected() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    configure_fake_pi(&mut runtime, &[]);
    let profile = profile();
    runtime.store_mut().insert_profile(&profile).unwrap();
    let mut stored_task = task();
    stored_task.session_file = Some(std::env::temp_dir().join("not-profile-session.jsonl"));
    runtime.store_mut().insert_task(&stored_task).unwrap();
    let error = runtime.start_task("task-runtime").unwrap_err();
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

pub(crate) fn active_history_timeout_falls_back_to_the_existing_journal() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    configure_fake_pi(&mut runtime, &[]);
    let profile = profile();
    std::fs::create_dir_all(&profile.session_dir).unwrap();
    runtime.store_mut().insert_profile(&profile).unwrap();
    let session_file = profile.session_dir.join("active-history.jsonl");
    let journal = [
        serde_json::json!({"type":"session", "id":"active-history"}),
        serde_json::json!({"type":"message", "message":{"role":"user", "content":"still here"}}),
        serde_json::json!({"type":"message", "message":{"role":"assistant", "content":"not empty"}}),
    ]
    .into_iter()
    .map(|entry| serde_json::to_string(&entry).unwrap())
    .collect::<Vec<_>>()
    .join("\n");
    std::fs::write(&session_file, format!("{journal}\n")).unwrap();
    let mut stored_task = task();
    stored_task.session_file = Some(session_file);
    runtime.store_mut().insert_task(&stored_task).unwrap();

    runtime.start_task("task-runtime").unwrap();
    let response = runtime.history("task-runtime").unwrap();
    assert_eq!(response["pending"], false);
    assert_eq!(response["data"]["messages"].as_array().unwrap().len(), 2);
    runtime.stop_task("task-runtime").unwrap();
}

pub(crate) fn startup_recovery_clears_ghost_running_and_restores_failed_error() {
    let mut runtime = DesktopRuntime::in_memory().unwrap();
    let profile = profile();
    std::fs::create_dir_all(&profile.session_dir).unwrap();
    runtime.store_mut().insert_profile(&profile).unwrap();

    let mut ghost = task();
    ghost.id = "ghost-running".to_string();
    ghost.status = TaskStatus::Running;
    ghost.updated_at = 41;
    runtime.store_mut().insert_task(&ghost).unwrap();

    let error_session = profile.session_dir.join("failed-network.jsonl");
    let journal = [
        serde_json::json!({"type":"session", "id":"failed-session"}),
        serde_json::json!({"type":"message", "message":{"role":"user", "content":[{"type":"text","text":"你好呀"}]}}),
        serde_json::json!({"type":"message", "message":{"role":"assistant", "content":[], "stopReason":"error", "errorMessage":"Unable to connect. Is the computer able to access the url?"}}),
    ]
    .into_iter()
    .map(|entry| serde_json::to_string(&entry).unwrap())
    .collect::<Vec<_>>()
    .join("\n");
    std::fs::write(&error_session, format!("{journal}\n")).unwrap();
    let mut failed = task();
    failed.id = "failed-network".to_string();
    failed.session_file = Some(error_session.clone());
    failed.updated_at = 42;
    runtime.store_mut().insert_task(&failed).unwrap();

    runtime.recover_stale_task_statuses().unwrap();
    let ghost = runtime.store().get_task("ghost-running").unwrap().unwrap();
    let failed = runtime.store().get_task("failed-network").unwrap().unwrap();
    assert_eq!(ghost.status, TaskStatus::Disconnected);
    assert_eq!(ghost.updated_at, 41);
    assert_eq!(failed.status, TaskStatus::Failed);
    assert_eq!(failed.updated_at, 42);
}
