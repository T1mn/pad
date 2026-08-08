mod core {
    use super::super::model::{ApiRequest, ApiResponse};
    use serde_json::json;

    pub(super) fn status_response() -> ApiResponse {
        ApiResponse::ok(
            "ok",
            Some(json!({
                "runtime": "native",
                "online": crate::runtime_status::read_status(&crate::paths::pad_status_path())
                    .is_some_and(|status| crate::runtime_status::process_alive(status.pid)),
            })),
        )
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
        ApiResponse::err(format!(
            "native pane {pane_id} is owned by the PAD UI; prompt dispatch requires the in-app terminal"
        ))
    }
}
mod remote {
    use super::super::model::{ApiRequest, ApiResponse};
    use serde_json::json;

    pub(super) fn browser_open_response(request: ApiRequest) -> ApiResponse {
        let Some(url) = request.url.as_deref() else {
            return ApiResponse::err("missing url");
        };
        if request.dry_run {
            return match crate::browser_remote::browser_open_command(url) {
                Ok(command) => ApiResponse::ok(
                    "dry_run",
                    Some(json!({ "program": command.program, "args": command.args })),
                ),
                Err(err) => ApiResponse::err(format!("browser command failed: {err}")),
            };
        }
        match crate::browser_remote::open_browser_url(url) {
            Ok(()) => ApiResponse::ok("opened", None),
            Err(err) => ApiResponse::err(format!("browser open failed: {err}")),
        }
    }

    pub(crate) fn remote_exec_command(request: &ApiRequest) -> Result<Vec<String>, ApiResponse> {
        let Some(host) = request.host.as_deref() else {
            return Err(ApiResponse::err("missing host"));
        };
        let Some(command) = request.command.as_deref() else {
            return Err(ApiResponse::err("missing command"));
        };
        crate::browser_remote::remote_ssh_command(&crate::browser_remote::RemoteCommandRequest {
            host: host.to_string(),
            cwd: request.cwd.clone(),
            command: command.to_string(),
        })
        .map_err(|err| ApiResponse::err(format!("invalid host: {err}")))
    }

    pub(super) fn remote_exec_response(request: ApiRequest) -> ApiResponse {
        let ssh = match remote_exec_command(&request) {
            Ok(ssh) => ssh,
            Err(response) => return response,
        };
        if request.dry_run {
            return ApiResponse::ok("dry_run", Some(json!({ "command": ssh })));
        }
        ApiResponse::err("live remote exec requires the async socket server")
    }
}

use super::model::{ApiRequest, ApiResponse};
use core::{inbox_response, mark_read_response, prompt_response, status_response};
pub(crate) use remote::remote_exec_command;
use remote::{browser_open_response, remote_exec_response};

pub fn handle_request(request: ApiRequest) -> ApiResponse {
    match request.action.as_str() {
        "status" => status_response(),
        "inbox" => inbox_response(),
        "mark_read" => mark_read_response(request),
        "prompt" => prompt_response(request),
        "browser_open" => browser_open_response(request),
        "remote_exec" => remote_exec_response(request),
        other => ApiResponse::err(format!("unknown action: {other}")),
    }
}
