use std::io;

use crate::tmux_capabilities::{probe_tmux_capabilities, TmuxProbeReport};

pub(super) fn ensure_tmux_compatible() -> io::Result<TmuxProbeReport> {
    let report = probe_tmux_capabilities().map_err(io::Error::other)?;
    let required = report.missing_required_capabilities();
    if required.is_empty() {
        return Ok(report);
    }

    Err(io::Error::other(compatibility_error_message(
        &report, &required,
    )))
}

fn compatibility_error_message(report: &TmuxProbeReport, required: &[&str]) -> String {
    let version = report.version_raw.trim();
    let mut message = format!(
        "tmux compatibility probe failed for `{}`. Missing required capabilities: {}.",
        version,
        join_display(required, ", ")
    );
    let optional = report.missing_optional_capabilities();
    if !optional.is_empty() {
        message.push_str(&format!(
            " Optional capabilities unavailable: {}.",
            join_display(&optional, ", ")
        ));
    }
    if !report.notes.is_empty() {
        message.push_str(&format!(
            " Probe notes: {}.",
            join_display(&report.notes, " | ")
        ));
    }
    message
}

fn join_display<T: AsRef<str>>(items: &[T], separator: &str) -> String {
    let mut joined = String::new();
    for item in items {
        if !joined.is_empty() {
            joined.push_str(separator);
        }
        joined.push_str(item.as_ref());
    }
    joined
}

#[cfg(test)]
#[path = "compatibility_tests.rs"]
mod tests;
