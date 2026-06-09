use super::{version::parse_tmux_version, TmuxCapabilities, TmuxProbeReport, TmuxVersion};

#[test]
fn parse_tmux_version_handles_suffix_minor() {
    assert_eq!(
        parse_tmux_version("tmux 3.5a"),
        Some(TmuxVersion {
            major: 3,
            minor: 5,
            patch: None,
            suffix: Some("a".to_string()),
        })
    );
}

#[test]
fn parse_tmux_version_handles_patch() {
    assert_eq!(
        parse_tmux_version("tmux 3.4.1"),
        Some(TmuxVersion {
            major: 3,
            minor: 4,
            patch: Some(1),
            suffix: None,
        })
    );
}

#[test]
fn report_separates_required_and_optional_capabilities() {
    let report = TmuxProbeReport {
        version_raw: "tmux 3.1".to_string(),
        version: None,
        capabilities: TmuxCapabilities {
            pane_metadata_formats: false,
            display_message_formats: true,
            root_key_table: true,
            literal_send_keys: false,
            bracketed_paste: false,
            control_mode_flags: false,
            focus_events: false,
        },
        notes: Vec::new(),
    };

    assert_eq!(
        report.missing_required_capabilities(),
        vec!["pane metadata formats", "send-keys -l"]
    );
    assert_eq!(
        report.missing_optional_capabilities(),
        vec![
            "control-mode attach flags",
            "bracketed paste",
            "focus-events"
        ]
    );
}

#[test]
fn report_summary_lines_include_notes() {
    let report = TmuxProbeReport {
        version_raw: " tmux 3.5a ".to_string(),
        version: None,
        capabilities: TmuxCapabilities {
            pane_metadata_formats: true,
            display_message_formats: true,
            root_key_table: true,
            literal_send_keys: true,
            bracketed_paste: false,
            control_mode_flags: true,
            focus_events: false,
        },
        notes: vec!["focus probe skipped".into()],
    };

    assert_eq!(
        report.summary_lines(),
        vec![
            "tmux version: tmux 3.5a".to_string(),
            "capabilities: pane-metadata=yes display-message=yes root-keys=yes send-keys-l=yes paste-p=no control-flags=yes focus-events=no".to_string(),
            "notes:".to_string(),
            "  - focus probe skipped".to_string(),
        ]
    );
}
