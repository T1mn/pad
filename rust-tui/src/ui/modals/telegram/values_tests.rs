use super::mask_secret;

#[test]
fn mask_secret_keeps_existing_empty_and_short_behavior() {
    assert_eq!(mask_secret(""), "(empty)");
    assert_eq!(mask_secret("abc"), "***");
    assert_eq!(mask_secret("abcdefghij"), "**********");
}

#[test]
fn mask_secret_keeps_first_and_last_four_for_long_values() {
    assert_eq!(mask_secret("abcdefghijkl"), "abcd…ijkl");
    assert_eq!(mask_secret("一二三四五六七八九十甲"), "一二三四…八九十甲");
}
