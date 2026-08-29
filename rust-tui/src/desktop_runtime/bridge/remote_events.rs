use super::{format, protocol, write_event_line, DesktopRequest, DesktopResponse};
use crate::desktop_runtime::DesktopRuntime;
use crate::pi_runtime::PiPoll;
use serde_json::{json, Value};
use std::io::{self, Write};

pub(super) fn publish_local_result_to_remote(
    runtime: &DesktopRuntime,
    request: &DesktopRequest,
    response: &DesktopResponse,
) {
    let action = request.action.as_deref().unwrap_or("");
    let Some(result) = response.result.clone() else {
        return;
    };
    if action == "poll" {
        if let Some(task_id) = request.task_id.as_deref() {
            runtime.publish_remote_task_event(task_id, "task_output", result);
        }
        return;
    }
    if !matches!(
        action,
        "create_task"
            | "start_task"
            | "retry_task"
            | "prompt"
            | "abort"
            | "stop"
            | "stop_task"
            | "respond_ui"
            | "set_task"
    ) {
        return;
    }
    let task_id = request
        .task_id
        .as_deref()
        .or_else(|| result.get("task_id").and_then(Value::as_str));
    if let Some(task_id) = task_id {
        runtime.publish_remote_task_event(
            task_id,
            "invalidated",
            json!({"action":action,"task_id":task_id}),
        );
    }
}

/// The sole Pi stdout consumer lives on the bridge owner thread. Its output
/// is fanned out to both transports before any subsequent poll can consume it.
pub(crate) fn pump_runtime_tasks(
    runtime: &mut DesktopRuntime,
    stdout: &mut impl Write,
    negotiated_v2: bool,
    sequence: &mut u64,
) -> io::Result<()> {
    for task_id in runtime.active_task_ids() {
        let Ok(poll) = runtime.poll_task(&task_id) else {
            continue;
        };
        if poll.is_empty() {
            continue;
        }
        let mut payload = task_output_payload(runtime, &task_id, &poll);
        runtime.publish_remote_task_event(&task_id, "task_output", payload.clone());
        if !negotiated_v2 || !task_is_locally_visible(runtime, &task_id) {
            continue;
        }
        if protocol::sanitize_v2_result(runtime, &mut payload).is_err() {
            continue;
        }
        *sequence = sequence.saturating_add(1);
        write_event_line(
            stdout,
            &protocol::event_frame(*sequence, "task_output", payload),
        )?;
    }
    Ok(())
}

pub(crate) fn task_output_payload(runtime: &DesktopRuntime, task_id: &str, poll: &PiPoll) -> Value {
    let snapshot = runtime.runtime_snapshot(task_id).ok().flatten();
    let task = runtime.store().get_task(task_id).ok().flatten();
    json!({
        "task_id": task_id,
        "poll": format::poll_value(poll),
        "runtime": snapshot.as_ref().map(format::snapshot_value),
        "task": task,
        // This top-level collection is authoritative. `poll` is only the
        // latest delta and must never clear an older pending interaction.
        "pending_ui_requests": runtime.pending_ui_requests(task_id),
    })
}

fn task_is_locally_visible(runtime: &DesktopRuntime, task_id: &str) -> bool {
    let active_profile = protocol::active_profile_id(runtime).ok().flatten();
    runtime
        .store()
        .get_task(task_id)
        .ok()
        .flatten()
        .is_some_and(|task| active_profile.as_deref() == Some(task.profile_id.as_str()))
}
