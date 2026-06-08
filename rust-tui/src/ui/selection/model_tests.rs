use super::SelectionItem;

#[test]
fn matches_query_checks_value_text() {
    let item = SelectionItem {
        title: "Theme".into(),
        value: Some("dark".into()),
        ..Default::default()
    };

    assert!(item.matches_query("dark"));
}

#[test]
fn matches_query_checks_ascii_case_without_lowercase_copy() {
    let item = SelectionItem {
        title: "Fast Mode".into(),
        keyword: Some("CodexRuntime".into()),
        ..Default::default()
    };

    assert!(item.matches_query("runtime"));
}

#[test]
fn matches_query_keeps_unicode_case_fold_behavior() {
    let item = SelectionItem {
        title: "Éclair Theme".into(),
        ..Default::default()
    };

    assert!(item.matches_query("éclair"));
}
