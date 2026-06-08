use super::compatibility_error_message;
use crate::tmux_capabilities::{TmuxCapabilities, TmuxProbeReport};

#[test]
fn compatibility_error_message_lists_required_optional_and_notes() {
    let report = TmuxProbeReport {
        version_raw: "tmux 3.1 ".to_string(),
        version: None,
        capabilities: TmuxCapabilities {
            pane_metadata_formats: false,
            display_message_formats: true,
            root_key_table: true,
            literal_send_keys: false,
            bracketed_paste: false,
            control_mode_flags: false,
            focus_events: true,
        },
        notes: vec!["probe one".to_string(), "probe two".to_string()],
    };

    assert_eq!(
        compatibility_error_message(
            &report,
            &["pane metadata formats", "send-keys -l"],
        ),
        "tmux compatibility probe failed for `tmux 3.1`. Missing required capabilities: pane metadata formats, send-keys -l. Optional capabilities unavailable: control-mode attach flags, bracketed paste. Probe notes: probe one | probe two."
    );
}
