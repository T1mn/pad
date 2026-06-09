use super::format_opencode_args;

#[test]
fn opencode_args_format_keeps_diagnostics_error_shape() {
    assert_eq!(
        format_opencode_args(&["models", "--verbose"]),
        "models --verbose"
    );
}
