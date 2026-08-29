use crate::permission_policy::{Profile, Task, TaskStatus};
use serde_json::json;
use std::time::Duration;

pub(crate) fn pending_ui_is_authoritative_after_event_consumption_and_clears_on_response() {
    let mut runtime = crate::desktop_runtime::DesktopRuntime::in_memory().unwrap();
    crate::desktop_runtime::tests::configure_fake_pi(
        &mut runtime,
        &[json!({
            "type":"extension_ui_request",
            "generation":1,
            "sequence":1,
            "id":"select-1",
            "method":"select",
            "title":"Resolve conflict",
            "options":["Keep local","Keep both"],
            "defaultIndex":1
        })],
    );
    runtime
        .create_profile(Profile {
            id: "a".to_string(),
            name: "A".to_string(),
            ..Profile::default()
        })
        .unwrap();
    runtime
        .create_task(Task {
            id: "a-task".to_string(),
            profile_id: "a".to_string(),
            cwd: std::env::temp_dir(),
            ..Task::default()
        })
        .unwrap();
    runtime.start_task("a-task").unwrap();
    for _ in 0..20 {
        let _ = runtime.poll_task("a-task").unwrap();
        if !runtime.pending_ui_requests("a-task").is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(runtime.pending_ui_requests("a-task")[0]["id"], "select-1");
    let later_delta = crate::pi_runtime::PiPoll {
        diagnostics: vec!["ordinary later delta".to_string()],
        ..Default::default()
    };
    let later_payload = crate::desktop_runtime::bridge::remote_events::task_output_payload(
        &runtime,
        "a-task",
        &later_delta,
    );
    assert_eq!(
        later_payload["pending_ui_requests"][0]["id"], "select-1",
        "a later delta must retain the authoritative pending interaction"
    );
    assert_eq!(
        runtime.store().get_task("a-task").unwrap().unwrap().status,
        TaskStatus::NeedsApproval
    );

    let bootstrap = runtime
        .execute_profile_remote_action_for_test("a", "bootstrap", json!({}))
        .unwrap();
    let pending = &bootstrap["pending_ui_requests_by_task"]["a-task"][0];
    assert_eq!(pending["id"], "select-1");
    assert_eq!(pending["kind"], "select");
    assert_eq!(pending["options"], json!(["Keep local", "Keep both"]));

    let snapshot = runtime
        .execute_profile_remote_action_for_test(
            "a",
            "runtime_snapshot",
            json!({"task_id":"a-task"}),
        )
        .unwrap();
    assert_eq!(snapshot["pending_ui_requests"][0]["id"], "select-1");
    let history = runtime
        .execute_profile_remote_action_for_test("a", "history", json!({"task_id":"a-task"}))
        .unwrap();
    assert_eq!(history["pending_ui_requests"][0]["id"], "select-1");

    runtime
        .execute_profile_remote_action_for_test(
            "a",
            "respond_ui",
            json!({
                "task_id":"a-task",
                "request_id":"select-1",
                "response_kind":"select",
                "value":"Keep both"
            }),
        )
        .unwrap();
    assert!(runtime.pending_ui_requests("a-task").is_empty());
    assert_eq!(
        runtime.runtime_snapshot("a-task").unwrap().unwrap().status,
        crate::pi_runtime::PiRuntimeStatus::Running
    );
    let refreshed = runtime
        .execute_profile_remote_action_for_test("a", "bootstrap", json!({}))
        .unwrap();
    assert!(refreshed["pending_ui_requests_by_task"]
        .as_object()
        .unwrap()
        .is_empty());
    runtime.stop_task("a-task").unwrap();
}

pub(crate) fn remote_prompt_starts_an_idle_task_before_delivery() {
    let mut runtime = crate::desktop_runtime::DesktopRuntime::in_memory().unwrap();
    crate::desktop_runtime::tests::configure_fake_pi(&mut runtime, &[]);
    runtime
        .create_profile(Profile {
            id: "a".to_string(),
            name: "A".to_string(),
            ..Profile::default()
        })
        .unwrap();
    runtime
        .create_task(Task {
            id: "idle-task".to_string(),
            profile_id: "a".to_string(),
            cwd: std::env::temp_dir(),
            ..Task::default()
        })
        .unwrap();
    assert!(!runtime.is_running("idle-task"));
    let result = runtime
        .execute_profile_remote_action_for_test(
            "a",
            "prompt",
            json!({"task_id":"idle-task","prompt":"first remote prompt"}),
        )
        .unwrap();
    assert_eq!(result["accepted"], true);
    assert!(runtime.is_running("idle-task"));
    runtime
        .active_tasks
        .get("idle-task")
        .unwrap()
        .supervisor
        .shutdown()
        .unwrap();
    let restarted = runtime
        .execute_profile_remote_action_for_test(
            "a",
            "prompt",
            json!({"task_id":"idle-task","prompt":"prompt after child exit"}),
        )
        .unwrap();
    assert_eq!(restarted["accepted"], true);
    runtime.stop_task("idle-task").unwrap();
}

pub(crate) fn expired_and_fire_and_forget_ui_events_never_become_ghost_prompts() {
    let mut runtime = crate::desktop_runtime::DesktopRuntime::in_memory().unwrap();
    crate::desktop_runtime::tests::configure_fake_pi(
        &mut runtime,
        &[
            json!({
                "type":"extension_ui_request", "generation":1, "sequence":1,
                "id":"notice-1", "method":"notify", "message":"done"
            }),
            json!({
                "type":"extension_ui_request", "generation":1, "sequence":2,
                "id":"short-1", "method":"confirm", "title":"Quick", "timeout":1
            }),
        ],
    );
    runtime
        .create_profile(Profile {
            id: "a".to_string(),
            ..Profile::default()
        })
        .unwrap();
    runtime
        .create_task(Task {
            id: "a-task".to_string(),
            profile_id: "a".to_string(),
            cwd: std::env::temp_dir(),
            ..Task::default()
        })
        .unwrap();
    runtime.start_task("a-task").unwrap();
    for _ in 0..20 {
        let _ = runtime.poll_task("a-task").unwrap();
        if !runtime.pending_ui_requests("a-task").is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let (owner_tx, _owner_rx) = std::sync::mpsc::sync_channel(4);
    runtime.attach_remote_gateway(owner_tx);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    runtime
        .remote_gateway
        .as_ref()
        .unwrap()
        .shared
        .lock()
        .unwrap()
        .broker
        .clients
        .push(super::test_client("a", sender));
    std::thread::sleep(Duration::from_millis(5));
    let mut stdout = Vec::new();
    let mut sequence = 0;
    crate::desktop_runtime::bridge::remote_events::pump_runtime_tasks(
        &mut runtime,
        &mut stdout,
        false,
        &mut sequence,
    )
    .unwrap();
    let expired = receiver.try_recv().expect("expiry must be fanned out");
    assert_eq!(expired.value["kind"], "task_output");
    assert_eq!(
        expired.value["payload"]["pending_ui_requests"],
        json!([]),
        "expiry must authoritatively clear the phone card"
    );
    assert!(runtime.pending_ui_requests("a-task").is_empty());
    assert_ne!(
        runtime.runtime_snapshot("a-task").unwrap().unwrap().status,
        crate::pi_runtime::PiRuntimeStatus::NeedsApproval
    );
    assert_ne!(
        runtime.store().get_task("a-task").unwrap().unwrap().status,
        TaskStatus::NeedsApproval
    );
    runtime.stop_task("a-task").unwrap();
}

pub(crate) fn delayed_remote_history_is_pending_and_never_clears_cached_messages() {
    let mut runtime = crate::desktop_runtime::DesktopRuntime::in_memory().unwrap();
    crate::desktop_runtime::tests::configure_fake_pi(&mut runtime, &[]);
    runtime
        .create_profile(Profile {
            id: "a".to_string(),
            ..Profile::default()
        })
        .unwrap();
    runtime
        .create_task(Task {
            id: "a-task".to_string(),
            profile_id: "a".to_string(),
            cwd: std::env::temp_dir(),
            ..Task::default()
        })
        .unwrap();
    runtime.start_task("a-task").unwrap();
    let history = runtime
        .execute_profile_remote_action_for_test("a", "history", json!({"task_id":"a-task"}))
        .unwrap();
    assert_eq!(history["pending"], true);
    assert!(history.get("messages").is_none());
    assert!(history.get("response").is_none());
    runtime.stop_task("a-task").unwrap();
}
