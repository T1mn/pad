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
