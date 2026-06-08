use std::io;

use crate::tmux_capabilities::{probe_tmux_capabilities, TmuxProbeReport};

pub(super) fn ensure_tmux_compatible() -> io::Result<TmuxProbeReport> {
    let report = probe_tmux_capabilities().map_err(io::Error::other)?;
    let required = report.missing_required_capabilities();
    if required.is_empty() {
        return Ok(report);
    }

    let version = report.version_raw.trim();
    let mut message = format!(
        "tmux compatibility probe failed for `{}`. Missing required capabilities: {}.",
        version,
        required.join(", ")
    );
    let optional = report.missing_optional_capabilities();
    if !optional.is_empty() {
        message.push_str(&format!(
            " Optional capabilities unavailable: {}.",
            optional.join(", ")
        ));
    }
    if !report.notes.is_empty() {
        message.push_str(&format!(" Probe notes: {}.", report.notes.join(" | ")));
    }
    Err(io::Error::other(message))
}
