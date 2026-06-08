use super::super::model::{ApiRequest, ApiResponse};
use serde_json::json;

pub(super) fn recipe_run_response(request: ApiRequest) -> ApiResponse {
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

pub(super) fn resume_response(request: ApiRequest) -> ApiResponse {
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
