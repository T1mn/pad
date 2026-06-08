use super::super::model::{ApiRequest, ApiResponse};
use serde_json::json;

pub(super) fn status_response() -> ApiResponse {
    match crate::scanner::scan_panels() {
        Ok(panels) => ApiResponse::ok(
            "ok",
            Some(json!({
                "panels": panels.into_iter().map(|panel| json!({
                    "pane_id": panel.pane_id,
                    "session": panel.session,
                    "window_index": panel.window_index,
                    "agent_type": panel.agent_type.to_string(),
                    "working_dir": panel.working_dir,
                    "state": format!("{:?}", panel.state),
                    "is_active": panel.is_active,
                    "agent_session_id": panel.agent_session_id,
                })).collect::<Vec<_>>()
            })),
        ),
        Err(err) => ApiResponse::err(format!("scan failed: {err}")),
    }
}

pub(super) fn inbox_response() -> ApiResponse {
    let inbox = crate::notification_inbox::load();
    ApiResponse::ok(
        "ok",
        Some(json!({
            "unread": inbox.unread_count(),
            "entries": inbox.entries,
        })),
    )
}

pub(super) fn mark_read_response(request: ApiRequest) -> ApiResponse {
    let Some(id) = request.id.as_deref() else {
        return ApiResponse::err("missing id");
    };
    match crate::notification_inbox::mark_read(id) {
        Ok(changed) => ApiResponse::ok("ok", Some(json!({ "changed": changed }))),
        Err(err) => ApiResponse::err(format!("mark_read failed: {err}")),
    }
}

pub(super) fn prompt_response(request: ApiRequest) -> ApiResponse {
    let Some(pane_id) = request.pane_id.as_deref() else {
        return ApiResponse::err("missing pane_id");
    };
    let Some(prompt) = request.prompt.as_deref() else {
        return ApiResponse::err("missing prompt");
    };
    if request.dry_run {
        return ApiResponse::ok(
            "dry_run",
            Some(json!({ "pane_id": pane_id, "prompt_len": prompt.chars().count() })),
        );
    }
    match crate::tmux_dispatch::dispatch_prompt(pane_id, prompt) {
        Ok(()) => ApiResponse::ok("prompt dispatched", None),
        Err(err) => ApiResponse::err(format!("prompt dispatch failed: {err}")),
    }
}
