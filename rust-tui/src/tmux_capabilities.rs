use std::process::Command;

mod probe;
mod report;
#[cfg(test)]
mod tests;
mod version;

use probe::{
    now_stamp, probe_tmux_capabilities_with_socket, start_probe_server, stop_probe_server,
};
pub use report::{TmuxCapabilities, TmuxProbeReport};
use version::parse_tmux_version;
pub use version::TmuxVersion;

pub fn read_tmux_version() -> Result<(String, Option<TmuxVersion>), String> {
    let output = Command::new("tmux")
        .arg("-V")
        .output()
        .map_err(|err| format!("failed to run `tmux -V`: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "`tmux -V` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((raw.clone(), parse_tmux_version(&raw)))
}

pub fn probe_tmux_capabilities() -> Result<TmuxProbeReport, String> {
    let (version_raw, version) = read_tmux_version()?;
    let socket_name = format!("pad-probe-{}-{}", std::process::id(), now_stamp());
    let mut notes = Vec::new();

    start_probe_server(&socket_name)
        .map_err(|err| format!("failed to start temporary tmux probe server: {err}"))?;

    let capabilities = probe_tmux_capabilities_with_socket(&socket_name, &mut notes);

    let _ = stop_probe_server(&socket_name);

    Ok(TmuxProbeReport {
        version_raw,
        version,
        capabilities,
        notes,
    })
}
