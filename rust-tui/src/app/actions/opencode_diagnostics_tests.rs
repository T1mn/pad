use super::{diagnostics_path, format_report, DiagnosticsSection};
use std::path::Path;

#[test]
fn diagnostics_path_uses_timestamped_txt() {
    assert_eq!(
        diagnostics_path(Path::new("/tmp/diag"), 42),
        Path::new("/tmp/diag/opencode-diagnostics-42.txt")
    );
}

#[test]
fn diagnostics_report_has_expected_sections() {
    let sections = [
        DiagnosticsSection {
            title: "version",
            body: "1.2.3\n".into(),
        },
        DiagnosticsSection {
            title: "debug info",
            body: "info\n".into(),
        },
        DiagnosticsSection {
            title: "debug paths",
            body: "paths\n".into(),
        },
        DiagnosticsSection {
            title: "debug config",
            body: "config\n".into(),
        },
        DiagnosticsSection {
            title: "providers list",
            body: "providers\n".into(),
        },
        DiagnosticsSection {
            title: "models --verbose",
            body: "models\n".into(),
        },
        DiagnosticsSection {
            title: "mcp list",
            body: "ERROR: no mcp\n".into(),
        },
    ];
    let body = format_report(&sections);
    assert!(body.contains("## version"));
    assert!(body.contains("1.2.3"));
    assert!(body.contains("## debug info"));
    assert!(body.contains("## debug paths"));
    assert!(body.contains("## debug config"));
    assert!(body.contains("## providers list"));
    assert!(body.contains("## models --verbose"));
    assert!(body.contains("## mcp list"));
    assert!(body.contains("ERROR: no mcp"));
}
