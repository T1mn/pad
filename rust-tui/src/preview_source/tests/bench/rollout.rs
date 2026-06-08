pub(super) fn rollout_session_id(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy();
    let stem = file_name.strip_suffix(".jsonl")?;
    let stem = stem.strip_prefix("rollout-")?;
    if stem.len() < 36 {
        return None;
    }
    let candidate = &stem[stem.len().saturating_sub(36)..];
    for (idx, byte) in candidate.bytes().enumerate() {
        if matches!(idx, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return None;
            }
        } else if !(byte as char).is_ascii_hexdigit() {
            return None;
        }
    }
    Some(candidate.to_string())
}

#[test]
fn rollout_session_id_extracts_uuid_suffix() {
    let path = Path::new("/tmp/rollout-extra-123e4567-e89b-12d3-a456-426614174000.jsonl");
    assert_eq!(
        rollout_session_id(path).as_deref(),
        Some("123e4567-e89b-12d3-a456-426614174000")
    );
}

#[test]
fn rollout_session_id_rejects_non_rollout_or_invalid_uuid() {
    assert!(rollout_session_id(Path::new("/tmp/session.jsonl")).is_none());
    assert!(rollout_session_id(Path::new("/tmp/rollout-not-a-uuid.jsonl")).is_none());
}
