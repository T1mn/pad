use super::{find_detach_key, find_f12_key};

#[test]
fn finds_raw_ctrl_q_detach_key() {
    assert_eq!(find_detach_key(b"abc\x11def", 0x11), Some(3));
}

#[test]
fn finds_xterm_encoded_ctrl_q_detach_key() {
    assert_eq!(find_detach_key(b"abc\x1b[27;5;113~def", 0x11), Some(3));
}

#[test]
fn finds_standard_and_modified_f12() {
    assert_eq!(find_f12_key(b"abc\x1b[24~def"), Some(3));
    assert_eq!(find_f12_key(b"abc\x1b[24;2~def"), Some(3));
}
