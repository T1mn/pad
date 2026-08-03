use super::rollout_date_parts;

#[test]
fn rollout_date_parts_reads_normal_timestamp() {
    let parts = rollout_date_parts("rollout-2026-03-27T14-05-10-abc.jsonl").unwrap();
    assert_eq!(parts, ("2026", "03", "27"));
}

#[test]
fn rollout_date_parts_rejects_multibyte_name_instead_of_panicking() {
    // 字节长度够 10，但按字节切 [0..4] 会切断「中」。
    let error = rollout_date_parts("rollout-中文中文abcdefghij").unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn rollout_date_parts_rejects_multibyte_day_with_valid_separators() {
    // bytes[4] 和 bytes[7] 都是 '-'，旧的分隔符校验放行，但 [8..10] 会切断「中」。
    let error = rollout_date_parts("rollout-2026-01-中15Txxxx").unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn rollout_date_parts_rejects_missing_prefix_and_short_stem() {
    assert!(rollout_date_parts("2026-03-27.jsonl").is_err());
    assert!(rollout_date_parts("rollout-2026").is_err());
}
