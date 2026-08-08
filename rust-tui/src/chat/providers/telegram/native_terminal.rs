use crate::socket_api::model::ApiRequest;
use std::io;

pub(super) fn dispatch_prompt(pane_id: &str, prompt: &str) -> io::Result<()> {
    send(ApiRequest {
        action: "prompt".to_string(),
        pane_id: Some(pane_id.to_string()),
        prompt: Some(prompt.to_string()),
        ..ApiRequest::default()
    })
    .map(|_| ())
}

pub(super) fn capture_pane_tail(pane_id: &str, lines: usize) -> io::Result<String> {
    let response = send(ApiRequest {
        action: "capture".to_string(),
        pane_id: Some(pane_id.to_string()),
        ..ApiRequest::default()
    })?;
    let content = response
        .data
        .and_then(|data| data.get("content").cloned())
        .and_then(|content| content.as_str().map(str::to_string))
        .unwrap_or_default();
    Ok(content
        .lines()
        .rev()
        .take(lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n"))
}

pub(super) fn send_escape(pane_id: &str) -> io::Result<()> {
    send(ApiRequest {
        action: "escape".to_string(),
        pane_id: Some(pane_id.to_string()),
        ..ApiRequest::default()
    })
    .map(|_| ())
}

pub(super) fn send_approval_key(pane_id: &str, key: &str) -> io::Result<()> {
    send(ApiRequest {
        action: "approval".to_string(),
        pane_id: Some(pane_id.to_string()),
        command: Some(key.to_string()),
        ..ApiRequest::default()
    })
    .map(|_| ())
}

fn send(request: ApiRequest) -> io::Result<crate::socket_api::model::ApiResponse> {
    let response = crate::socket_api::client::send_request(&request)?;
    if response.ok {
        Ok(response)
    } else {
        Err(io::Error::other(response.message))
    }
}
