use super::{opencode_export_path, safe_filename, ExportMode};
use std::path::Path;

#[test]
fn opencode_export_path_sanitizes_session_id() {
    assert_eq!(
        opencode_export_path("ses/../abc def", Path::new("/tmp/out"), ExportMode::Raw),
        Path::new("/tmp/out/ses_abc_def.json")
    );
}

#[test]
fn opencode_sanitized_export_path_uses_distinct_suffix() {
    assert_eq!(
        opencode_export_path("ses_123", Path::new("/tmp/out"), ExportMode::Sanitized),
        Path::new("/tmp/out/ses_123.sanitized.json")
    );
}

#[test]
fn safe_filename_falls_back_for_empty_value() {
    assert_eq!(safe_filename("***"), "session");
}
