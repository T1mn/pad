use super::format_session_diag_message;

#[test]
fn diag_message_includes_empty_state_metadata() {
    let context = super::super::super::SessionDiagContext {
        target_label: "CODEX • demo".into(),
        pane_id: Some("%7".into()),
        request_id: Some("tg-7".into()),
        session_id: Some("session-7".into()),
        transcript_path: Some("/tmp/session.jsonl".into()),
        continuity: None,
    };

    let body = format_session_diag_message(crate::i18n::Locale::En, &context);
    assert!(body.contains("Session Diagnostic"));
    assert!(body.contains("CODEX • demo"));
    assert!(body.contains("Request: tg-7"));
    assert!(body.contains("Pane: %7"));
    assert!(body.contains("Session: session-7"));
    assert!(body.contains("No continuity diagnostic data is available yet."));
    assert!(body.contains("Transcript: /tmp/session.jsonl"));
}
