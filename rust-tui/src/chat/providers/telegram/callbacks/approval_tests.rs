use super::approval_prompt_text;

#[test]
fn approval_prompt_text_includes_metadata_and_justification() {
    let mut pending =
        crate::chat::providers::telegram::tests::sample_pending("tg-1", "%7", "awaiting_confirm");
    pending.session_id = Some("session-7".into());
    let request = crate::chat::approval::CodexApprovalRequest {
        call_id: "call-1".into(),
        justification: "Need to run cargo test".into(),
    };

    let body = approval_prompt_text(crate::i18n::Locale::En, &pending, &request);
    assert!(body.contains("Codex needs approval"));
    assert!(body.contains("Target: CODEX • 7"));
    assert!(body.contains("Request: tg-1"));
    assert!(body.contains("Pane: %7"));
    assert!(body.contains("Session: session-7"));
    assert!(body.ends_with("Need to run cargo test"));
}
