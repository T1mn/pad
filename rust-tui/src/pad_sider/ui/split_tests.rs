use super::left_column_width;

#[test]
fn keeps_left_column_stable_as_sider_grows() {
    assert_eq!(left_column_width(100), 34);
    assert_eq!(left_column_width(130), 41);
    assert_eq!(left_column_width(180), 46);
}

#[test]
fn avoids_over_compressing_left_column_when_narrow() {
    assert_eq!(left_column_width(70), 34);
    assert_eq!(left_column_width(60), 30);
    assert_eq!(left_column_width(50), 24);
}
