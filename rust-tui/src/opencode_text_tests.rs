use super::{extract_any_part_text, extract_display_part_text, message_role, OpenCodeRole};

pub(crate) fn joins_nested_text_parts_without_empty_items() {
    let raw = r#"{"text":["hello",{"content":"world"},"   "]}"#;

    assert_eq!(extract_any_part_text(raw), Some("hello\nworld".to_string()));
}

pub(crate) fn display_text_skips_non_display_part_types() {
    let raw = r#"{"type":"file","text":"hidden"}"#;

    assert_eq!(extract_display_part_text(raw), None);
}

pub(crate) fn reads_supported_message_roles() {
    assert_eq!(message_role(r#"{"role":"user"}"#), Some(OpenCodeRole::User));
    assert_eq!(
        message_role(r#"{"role":"assistant"}"#),
        Some(OpenCodeRole::Assistant)
    );
    assert_eq!(message_role(r#"{"role":"system"}"#), None);
}
