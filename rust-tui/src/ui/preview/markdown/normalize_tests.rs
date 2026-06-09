use super::normalize_session_detail_markdown;

#[test]
fn inserts_paragraph_gaps_between_plain_lines() {
    assert_eq!(
        normalize_session_detail_markdown("first\nsecond"),
        "first\n\nsecond"
    );
}

#[test]
fn keeps_fenced_code_lines_together() {
    assert_eq!(
        normalize_session_detail_markdown("```rs\nlet a = 1;\nlet b = 2;\n```"),
        "```rs\nlet a = 1;\nlet b = 2;\n```"
    );
}
