use super::*;

#[test]
fn rejects_unknown_action() {
    let response = handle_request(ApiRequest {
        action: "missing".into(),
        ..ApiRequest::default()
    });
    assert!(!response.ok);
}

#[test]
fn browser_open_dry_run_returns_command() {
    let response = handle_request(ApiRequest {
        action: "browser_open".into(),
        url: Some("http://localhost:3000".into()),
        dry_run: true,
        ..ApiRequest::default()
    });
    assert!(response.ok);
}

#[test]
fn prompt_dry_run_does_not_touch_tmux() {
    let response = handle_request(ApiRequest {
        action: "prompt".into(),
        pane_id: Some("%1".into()),
        prompt: Some("hello".into()),
        dry_run: true,
        ..ApiRequest::default()
    });
    assert!(response.ok);
}
