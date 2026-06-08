use super::TmuxCapabilities;

mod control;
mod formats;
mod input;
mod runtime;

use control::{probe_control_mode_flags, probe_focus_events, probe_root_key_table};
use formats::{probe_display_message_formats, probe_pane_metadata_formats};
use input::{probe_bracketed_paste, probe_literal_send_keys};
pub(super) use runtime::{now_stamp, start_probe_server, stop_probe_server};

pub(super) fn probe_tmux_capabilities_with_socket(
    socket_name: &str,
    notes: &mut Vec<String>,
) -> TmuxCapabilities {
    TmuxCapabilities {
        pane_metadata_formats: probe_pane_metadata_formats(socket_name, notes),
        display_message_formats: probe_display_message_formats(socket_name, notes),
        root_key_table: probe_root_key_table(socket_name, notes),
        literal_send_keys: probe_literal_send_keys(socket_name, notes),
        bracketed_paste: probe_bracketed_paste(socket_name, notes),
        control_mode_flags: probe_control_mode_flags(socket_name, notes),
        focus_events: probe_focus_events(socket_name, notes),
    }
}
