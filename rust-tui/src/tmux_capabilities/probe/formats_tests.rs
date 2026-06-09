use super::{display_message_probe_output_ok, pane_metadata_probe_output_ok};

#[test]
fn pane_metadata_probe_output_requires_exact_fields() {
    assert!(pane_metadata_probe_output_ok(
        "pad-probe|%1|12345|zsh|/tmp/demo"
    ));
    assert!(!pane_metadata_probe_output_ok(
        "pad-probe|%1|12345|zsh|/tmp/demo|extra"
    ));
    assert!(!pane_metadata_probe_output_ok("pad-probe|%1|12345|zsh|"));
}

#[test]
fn display_message_probe_output_requires_expected_fields() {
    assert!(display_message_probe_output_ok("pad-probe:0|1|%1"));
    assert!(!display_message_probe_output_ok("pad-probe:0|2|%1"));
    assert!(!display_message_probe_output_ok("pad-probe:0|1|%1|extra"));
}
