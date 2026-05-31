use super::model::{ApiRequest, ApiResponse};
use serde_json::json;
use std::process::Command;

pub fn handle_request(request: ApiRequest) -> ApiResponse {
    match request.action.as_str() {
        "status" => status_response(),
        "inbox" => inbox_response(),
        "mark_read" => mark_read_response(request),
        "prompt" => prompt_response(request),
        "recipe_run" => recipe_run_response(request),
        "resume" => resume_response(request),
        "browser_open" => browser_open_response(request),
        "remote_exec" => remote_exec_response(request),
        other => ApiResponse::err(format!("unknown action: {other}")),
    }
}

fn status_response() -> ApiResponse {
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

fn inbox_response() -> ApiResponse {
    let inbox = crate::notification_inbox::load();
    ApiResponse::ok(
        "ok",
        Some(json!({
            "unread": inbox.unread_count(),
            "entries": inbox.entries,
        })),
    )
}

fn mark_read_response(request: ApiRequest) -> ApiResponse {
    let Some(id) = request.id.as_deref() else {
        return ApiResponse::err("missing id");
    };
    match crate::notification_inbox::mark_read(id) {
        Ok(changed) => ApiResponse::ok("ok", Some(json!({ "changed": changed }))),
        Err(err) => ApiResponse::err(format!("mark_read failed: {err}")),
    }
}

fn prompt_response(request: ApiRequest) -> ApiResponse {
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

fn recipe_run_response(request: ApiRequest) -> ApiResponse {
    let Some(name) = request.name.as_deref() else {
        return ApiResponse::err("missing name");
    };
    let file = match crate::workspace_recipe::load() {
        Ok(file) => file,
        Err(err) => return ApiResponse::err(format!("recipe load failed: {err}")),
    };
    let Some(recipe) = crate::workspace_recipe::find_recipe(&file.recipes, name) else {
        return ApiResponse::err(format!("recipe not found: {name}"));
    };
    match crate::workspace_recipe::run_recipe(recipe, request.dry_run) {
        Ok(report) => ApiResponse::ok(
            if request.dry_run {
                "dry_run"
            } else {
                "launched"
            },
            Some(json!({
                "session_name": report.plan.session_name,
                "commands": report.plan.commands.iter().map(crate::workspace_recipe::display_command).collect::<Vec<_>>(),
                "browser_urls": report.plan.browser_urls,
                "executed": report.executed,
            })),
        ),
        Err(err) => ApiResponse::err(format!("recipe run failed: {err}")),
    }
}

fn resume_response(request: ApiRequest) -> ApiResponse {
    let Some(session_id) = request.session_id.as_deref() else {
        return ApiResponse::err("missing session_id");
    };
    let Some(target) = crate::agent_resume::find_resume_target(session_id) else {
        return ApiResponse::err(format!("resume target not found: {session_id}"));
    };
    match crate::agent_resume::launch_resume_target(&target, request.dry_run) {
        Ok(plan) => ApiResponse::ok(
            if request.dry_run {
                "dry_run"
            } else {
                "resumed"
            },
            Some(json!({
                "tmux_session_name": plan.tmux_session_name,
                "resume_command": plan.resume_command,
            })),
        ),
        Err(err) => ApiResponse::err(format!("resume failed: {err}")),
    }
}

fn browser_open_response(request: ApiRequest) -> ApiResponse {
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

fn remote_exec_response(request: ApiRequest) -> ApiResponse {
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

#[cfg(test)]
#[path = "handler_tests.rs"]
mod handler_tests;
