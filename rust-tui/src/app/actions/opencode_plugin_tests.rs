use super::{normalize_plugin_module, plugin_command};
use std::ffi::OsString;

#[test]
fn plugin_module_accepts_npm_names_scope_and_versions() {
    assert_eq!(
        normalize_plugin_module("opencode-foo").unwrap(),
        "opencode-foo"
    );
    assert_eq!(
        normalize_plugin_module("'@scope/opencode-plugin@1.2.3'").unwrap(),
        "@scope/opencode-plugin@1.2.3"
    );
}

#[test]
fn plugin_module_rejects_empty_multiline_flags_and_whitespace() {
    assert!(normalize_plugin_module(" ").is_err());
    assert!(normalize_plugin_module("a\nb").is_err());
    assert!(normalize_plugin_module("--global").is_err());
    assert!(normalize_plugin_module("opencode plugin").is_err());
    assert!(normalize_plugin_module("pkg;rm").is_err());
}

#[test]
fn plugin_command_quotes_configured_command_and_module() {
    assert_eq!(
        plugin_command(
            "@scope/opencode-plugin@1.2.3",
            &OsString::from("/opt/open code/bin/opencode"),
        ),
        "'/opt/open code/bin/opencode' plugin '@scope/opencode-plugin@1.2.3'"
    );
}
