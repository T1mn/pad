use super::collapse_whitespace;

#[test]
fn collapse_whitespace_joins_non_empty_parts_with_single_spaces() {
    assert_eq!(
        collapse_whitespace(" alpha\tbeta\n gamma  "),
        "alpha beta gamma"
    );
}

#[test]
fn collapse_whitespace_returns_empty_for_blank_input() {
    assert_eq!(collapse_whitespace(" \n\t "), "");
}
