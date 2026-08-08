use super::App;
use crate::socket_api::model::{ApiRequest, ApiResponse};
use serde_json::json;

impl App {
    pub fn drain_socket_api_requests(&mut self) {
        let mut calls = Vec::new();
        let mut disconnected = false;
        if let Some(receiver) = self.api_rx.as_mut() {
            loop {
                match receiver.try_recv() {
                    Ok(call) => calls.push(call),
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        if disconnected {
            self.api_rx = None;
        }
        for call in calls {
            if !call.try_begin() {
                continue;
            }
            let response = self.handle_socket_api_request(call.request);
            let _ = call.response.send(response);
        }
    }

    fn handle_socket_api_request(&mut self, request: ApiRequest) -> ApiResponse {
        match request.action.as_str() {
            "status" => self.native_status_response(),
            "prompt" => self.native_prompt_response(request),
            "capture" => self.native_capture_response(request),
            "escape" => self.native_key_response(request, vec![0x1b]),
            "approval" => {
                let Some(key) = request.command.as_deref() else {
                    return ApiResponse::err("missing approval key");
                };
                if !matches!(key, "y" | "a" | "n") {
                    return ApiResponse::err("approval key must be y, a, or n");
                }
                let bytes = key.as_bytes().to_vec();
                self.native_key_response(request, bytes)
            }
            other => ApiResponse::err(format!("unsupported UI action: {other}")),
        }
    }

    fn native_status_response(&self) -> ApiResponse {
        ApiResponse::ok(
            "ok",
            Some(json!({
                "runtime": "native",
                "panels": self.panels.iter().map(|panel| json!({
                    "pane_id": panel.pane_id,
                    "session": panel.session,
                    "window": panel.window,
                    "window_index": panel.window_index,
                    "pane": panel.pane,
                    "agent_type": panel.agent_type.to_string(),
                    "working_dir": panel.working_dir,
                    "state": format!("{:?}", panel.state).to_ascii_lowercase(),
                    "is_active": panel.is_active,
                    "transcript_path": panel.transcript_path,
                    "agent_session_id": panel.agent_session_id,
                })).collect::<Vec<_>>()
            })),
        )
    }

    fn native_prompt_response(&mut self, request: ApiRequest) -> ApiResponse {
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
        let mut bytes = prompt.as_bytes().to_vec();
        bytes.push(b'\r');
        match self.send_native_pane_input(pane_id, bytes) {
            Ok(()) => ApiResponse::ok("prompt dispatched", None),
            Err(error) => ApiResponse::err(format!("prompt dispatch failed: {error}")),
        }
    }

    fn native_capture_response(&self, request: ApiRequest) -> ApiResponse {
        let Some(pane_id) = request.pane_id.as_deref() else {
            return ApiResponse::err("missing pane_id");
        };
        match self.native_pane_text(pane_id) {
            Some(content) => ApiResponse::ok("ok", Some(json!({ "content": content }))),
            None => ApiResponse::err("native terminal pane has no frame"),
        }
    }

    fn native_key_response(&mut self, request: ApiRequest, bytes: Vec<u8>) -> ApiResponse {
        let Some(pane_id) = request.pane_id.as_deref() else {
            return ApiResponse::err("missing pane_id");
        };
        match self.send_native_pane_input(pane_id, bytes) {
            Ok(()) => ApiResponse::ok("input dispatched", None),
            Err(error) => ApiResponse::err(format!("input dispatch failed: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_is_served_from_the_native_ui_state() {
        let mut app = App::new();
        let response = app.handle_socket_api_request(ApiRequest {
            action: "status".to_string(),
            ..ApiRequest::default()
        });

        assert!(response.ok);
        let data = response.data.unwrap();
        assert_eq!(data["runtime"], "native");
        assert_eq!(data["panels"], serde_json::json!([]));
    }

    #[test]
    fn prompt_dry_run_does_not_require_a_live_pane() {
        let mut app = App::new();
        let response = app.handle_socket_api_request(ApiRequest {
            action: "prompt".to_string(),
            pane_id: Some("native:42".to_string()),
            prompt: Some("hello".to_string()),
            dry_run: true,
            ..ApiRequest::default()
        });

        assert!(response.ok);
        assert_eq!(response.data.unwrap()["prompt_len"], 5);
    }

    #[test]
    fn approval_rejects_keys_outside_the_explicit_allowlist() {
        let mut app = App::new();
        let response = app.handle_socket_api_request(ApiRequest {
            action: "approval".to_string(),
            pane_id: Some("native:42".to_string()),
            command: Some("x".to_string()),
            ..ApiRequest::default()
        });

        assert!(!response.ok);
        assert!(response.message.contains("y, a, or n"));
    }
}
