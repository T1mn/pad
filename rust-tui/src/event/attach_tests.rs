use super::bindings::{restore_binding_cmd, PAD_SIDER_TOGGLE_KEYS};

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
