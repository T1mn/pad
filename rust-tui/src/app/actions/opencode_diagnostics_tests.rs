use super::{diagnostics_path, format_report};
use std::path::Path;

#[test]
fn diagnostics_path_uses_timestamped_txt() {
    assert_eq!(
        diagnostics_path(Path::new("/tmp/diag"), 42),
        Path::new("/tmp/diag/opencode-diagnostics-42.txt")
    );
}

#[test]
fn diagnostics_report_has_provider_and_model_sections() {
    let body = format_report("providers\n", "models\n");
    assert!(body.contains("## providers list"));
    assert!(body.contains("providers"));
    assert!(body.contains("## models --verbose"));
    assert!(body.contains("models"));
}
