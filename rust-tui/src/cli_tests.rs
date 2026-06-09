use super::*;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn detects_internal_command_prefix() {
    assert!(is_internal_command(&args(&[
        "pad",
        "__internal",
        "pad-sider"
    ])));
    assert!(!is_internal_command(&args(&["pad", "telegram-bot"])));
}

#[test]
fn format_list_keeps_comma_separated_doctor_output() {
    assert_eq!(
        format_list(&["pane metadata formats", "send-keys -l"], ", "),
        "pane metadata formats, send-keys -l"
    );
}
