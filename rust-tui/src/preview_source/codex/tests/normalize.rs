use super::super::normalize_codex_user_text;

#[test]
fn normalize_codex_user_text_handles_image_only_message() {
    let text = "<image name=[Image #1]>\n</image>\n[Image #1]";
    assert_eq!(normalize_codex_user_text(text, Some(1)), "[Image x1]");
}

#[test]
fn normalize_codex_user_text_does_not_touch_plain_text_without_images() {
    let text = "literal [Image #1] text";
    assert_eq!(normalize_codex_user_text(text, None), text);
}

#[test]
fn normalize_codex_user_text_filters_environment_context_block() {
    let text = "<environment_context>\n  <cwd>/tmp/demo</cwd>\n</environment_context>";
    assert_eq!(normalize_codex_user_text(text, None), "");
}

#[test]
fn normalize_codex_user_text_strips_embedded_environment_context_block() {
    let text = "请分析一下\n<environment_context>\n  <cwd>/tmp/demo</cwd>\n</environment_context>\n这段结构";
    assert_eq!(
        normalize_codex_user_text(text, None),
        "请分析一下\n\n这段结构"
    );
}

#[test]
fn normalize_codex_user_text_filters_turn_aborted_marker() {
    let text = "<turn_aborted>\ninterrupted\n</turn_aborted>";
    assert_eq!(normalize_codex_user_text(text, None), "");
}

#[test]
fn normalize_codex_user_text_summarizes_user_shell_command() {
    let text = "<user_shell_command>\n<command>\necho hi\n</command>\n<result>\nExit code: 0\n</result>\n</user_shell_command>";
    assert_eq!(normalize_codex_user_text(text, None), "[shell] echo hi");
}
