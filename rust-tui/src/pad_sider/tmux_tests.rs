use crate::pad_sider::sizing::default_width;

#[test]
fn helper_uses_half_width() {
    assert_eq!(default_width(), "50%");
}
