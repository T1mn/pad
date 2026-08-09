use super::{mask_secret_prefix, trailing_chars, truncate_modal_line_middle};

pub(crate) fn truncate_modal_line_middle_keeps_existing_shape() {
    assert_eq!(truncate_modal_line_middle("abcdefghijkl", 8), "ab...jkl");
    assert_eq!(
        truncate_modal_line_middle("一二三四五六七八", 7),
        "一二...七八"
    );
}

pub(crate) fn truncate_modal_line_middle_handles_short_width() {
    assert_eq!(truncate_modal_line_middle("abcd", 3), "...");
    assert_eq!(truncate_modal_line_middle("abcde", 4), "...e");
}

pub(crate) fn trailing_chars_handles_zero_count() {
    assert_eq!(trailing_chars("abcd", 0), "");
}

pub(crate) fn mask_secret_prefix_keeps_ascii_behavior() {
    assert_eq!(mask_secret_prefix("   ", 12), "-");
    assert_eq!(mask_secret_prefix("sk-short", 12), "sk-short");
    assert_eq!(
        mask_secret_prefix("sk-0123456789abcdef", 12),
        "sk-012345678..."
    );
}

pub(crate) fn mask_secret_prefix_cuts_multibyte_secret_on_char_boundary() {
    // 字节长度 23，按字节切 [..12] 会切断「密」并 panic。
    assert_eq!(
        mask_secret_prefix("sk-a测试密钥abcdefg", 12),
        "sk-a测试密钥abcd..."
    );
    assert_eq!(mask_secret_prefix("ab测试令牌xyz", 10), "ab测试令牌xyz");
}
