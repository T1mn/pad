use super::should_restore_target_zoom;

#[test]
fn zoom_restore_option_only_restores_explicit_zoomed_targets() {
    assert!(should_restore_target_zoom(Some("1".into())));
    assert!(!should_restore_target_zoom(Some("0".into())));
    assert!(!should_restore_target_zoom(None));
}
