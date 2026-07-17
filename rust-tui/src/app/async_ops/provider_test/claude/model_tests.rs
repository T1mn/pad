use super::expanded_model_candidates;

#[test]
fn opus_1m_prefers_claude_code_wire_model() {
    assert_eq!(
        expanded_model_candidates("opus[1m]"),
        vec!["claude-opus-4-8", "opus[1m]"]
    );
}

#[test]
fn full_model_1m_strips_display_suffix_first() {
    assert_eq!(
        expanded_model_candidates("claude-opus-4-8[1m]"),
        vec!["claude-opus-4-8", "claude-opus-4-8[1m]"]
    );
}
