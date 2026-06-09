use super::single_quote;

#[test]
fn single_quote_wraps_plain_value() {
    assert_eq!(single_quote("alpha"), "'alpha'");
}

#[test]
fn single_quote_escapes_embedded_single_quotes() {
    assert_eq!(single_quote("bob's app"), r#"'bob'\''s app'"#);
}
