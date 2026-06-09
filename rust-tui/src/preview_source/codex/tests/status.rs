use super::super::status_probe::{extract_status_session_id, normalize_probe_capture};

#[test]
fn status_probe_extracts_uuid_like_session_id() {
    let capture = "Codex status\nSession ID: 123e4567-e89b-12d3-a456-426614174000\nIdle";
    assert_eq!(
        extract_status_session_id(capture),
        Some("123e4567-e89b-12d3-a456-426614174000".to_string())
    );
}

#[test]
fn status_probe_normalizes_capture_without_collecting_lines() {
    let capture = "  first  \nsecond\t \n\n";

    assert_eq!(normalize_probe_capture(capture), "first\nsecond");
}
