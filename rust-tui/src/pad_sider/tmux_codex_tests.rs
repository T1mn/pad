use super::is_codex_command;

#[test]
fn codex_command_detection_is_ascii_case_insensitive() {
    assert!(is_codex_command("codex"));
    assert!(is_codex_command("CODEX"));
    assert!(is_codex_command("/opt/bin/pad-codex"));
    assert!(!is_codex_command("claude"));
}
