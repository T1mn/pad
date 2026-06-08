use super::extract_subagent_notification_summary;

#[test]
fn subagent_summary_compacts_whitespace_without_losing_detail() {
    let text = concat!(
        "<subagent_notification>\n",
        r#"{"agent_path":"/tmp/audit","status":{"completed":"  first   line\nsecond line"}}"#,
        "\n</subagent_notification>"
    );

    let summary = extract_subagent_notification_summary(text).unwrap();
    assert_eq!(summary, "[subagent/completed] audit\nfirst line");
}
