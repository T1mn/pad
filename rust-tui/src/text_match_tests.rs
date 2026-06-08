use super::{contains_ascii_ignore_case, contains_ignore_case};

#[test]
fn ascii_contains_ignores_case_without_unicode_fold() {
    assert!(contains_ascii_ignore_case("/usr/bin/CODEX", "codex"));
    assert!(!contains_ascii_ignore_case("claude", "codex"));
}

#[test]
fn unicode_contains_keeps_case_fold_behavior() {
    assert!(contains_ignore_case("Éclair Theme", "éclair"));
}
