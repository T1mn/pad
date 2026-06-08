mod core;
mod launch;
mod remote;

use super::model::{ApiRequest, ApiResponse};
use core::{inbox_response, mark_read_response, prompt_response, status_response};
use launch::{recipe_run_response, resume_response};
use remote::{browser_open_response, remote_exec_response};

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

#[cfg(test)]
#[path = "handler_tests.rs"]
mod handler_tests;
