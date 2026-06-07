use super::bindings::{restore_binding_cmd, PAD_SIDER_TOGGLE_KEYS};
use super::status::desired_status_override;

#[test]
fn keep_status_inherits_visible_status_from_pad_session() {
    assert_eq!(
        desired_status_override("keep", "off", Some("on")).as_deref(),
        Some("on")
    );
}

#[test]
fn keep_status_noops_when_status_already_matches_pad_session() {
    assert_eq!(desired_status_override("keep", "on", Some("on")), None);
}

#[test]
fn keep_status_noops_without_pad_status() {
    assert_eq!(desired_status_override("keep", "off", None), None);
    assert_eq!(desired_status_override("keep", "off", Some("")), None);
}

#[test]
fn sider_toggle_keys_include_ctrl_tab() {
    assert!(PAD_SIDER_TOGGLE_KEYS.contains(&"F10"));
    assert!(PAD_SIDER_TOGGLE_KEYS.contains(&"C-Tab"));
}

#[test]
fn restore_binding_cmd_can_unbind_ctrl_tab() {
    assert_eq!(
        restore_binding_cmd(None, "C-Tab"),
        "tmux unbind-key -T root C-Tab"
    );
}
