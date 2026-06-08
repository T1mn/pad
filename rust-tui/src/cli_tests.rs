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
