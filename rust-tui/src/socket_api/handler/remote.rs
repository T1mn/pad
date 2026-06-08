use super::super::model::{ApiRequest, ApiResponse};
use serde_json::json;
use std::process::Command;

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

pub(super) fn remote_exec_response(request: ApiRequest) -> ApiResponse {
    let Some(host) = request.host else {
        return ApiResponse::err("missing host");
    };
    let Some(command) = request.command else {
        return ApiResponse::err("missing command");
    };
    let ssh =
        crate::browser_remote::remote_ssh_command(&crate::browser_remote::RemoteCommandRequest {
            host,
            cwd: request.cwd,
            command,
        });
    if request.dry_run {
        return ApiResponse::ok("dry_run", Some(json!({ "command": ssh })));
    }
    match Command::new(&ssh[0]).args(&ssh[1..]).output() {
        Ok(output) if output.status.success() => ApiResponse::ok(
            "ok",
            Some(json!({ "stdout": String::from_utf8_lossy(&output.stdout) })),
        ),
        Ok(output) => ApiResponse::err(format!(
            "remote exec failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(err) => ApiResponse::err(format!("remote exec failed: {err}")),
    }
}
