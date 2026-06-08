use super::should_retry_without_flags;

#[test]
fn retries_without_flags_for_legacy_tmux_attach_error() {
    assert!(should_retry_without_flags("tmux: unknown option -- f"));
}

#[test]
fn does_not_retry_for_other_control_mode_failures() {
    assert!(!should_retry_without_flags("no current client"));
}
