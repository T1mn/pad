use std::fs;
use std::io;
use std::path::Path;

use super::{claude_hook_bridge_path, codex_hook_bridge_path};

mod template;
pub(super) use template::{claude_hook_bridge_template, codex_hook_bridge_template};

pub(super) const CLAUDE_BRIDGE_VERSION: &str = "claude-2026-04-08.1";
pub(super) const CODEX_BRIDGE_VERSION: &str = "codex-2026-06-02.1";
const BRIDGE_VERSION_PREFIX: &str = "# pad-bridge-version: ";

pub(super) fn install_bridge_scripts() -> io::Result<()> {
    let claude_bridge = claude_hook_bridge_template();
    let codex_bridge = codex_hook_bridge_template();
    install_bridge_script(&claude_hook_bridge_path(), claude_bridge.as_str(), false)?;
    install_bridge_script(&codex_hook_bridge_path(), codex_bridge.as_str(), true)?;
    Ok(())
}

pub(super) fn log_bridge_statuses() {
    log_bridge_status(
        "claude",
        &claude_hook_bridge_path(),
        CLAUDE_BRIDGE_VERSION,
        false,
    );
    log_bridge_status(
        "codex",
        &codex_hook_bridge_path(),
        CODEX_BRIDGE_VERSION,
        true,
    );
}

fn install_bridge_script(path: &Path, desired: &str, require_turn_id: bool) -> io::Result<()> {
    let existing = fs::read_to_string(path).ok();

    if existing.as_deref() != Some(desired) {
        fs::write(path, desired)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    let actual = fs::read_to_string(path)?;
    if actual != desired {
        return Err(io::Error::other(format!(
            "bridge script verify failed at {}",
            path.display()
        )));
    }
    if require_turn_id && !actual.contains("\"turn_id\": payload.get(\"turn_id\")") {
        return Err(io::Error::other(format!(
            "bridge script missing turn_id forwarding at {}",
            path.display()
        )));
    }

    Ok(())
}

fn log_bridge_status(role: &str, path: &Path, expected_version: &str, expect_turn_id: bool) {
    match fs::read_to_string(path) {
        Ok(content) => {
            let actual_version = bridge_version(&content).unwrap_or("missing");
            let has_turn_id = content.contains("\"turn_id\": payload.get(\"turn_id\")");
            crate::log_debug!(
                "runtime_layout: bridge role={} path={} expected_version={} actual_version={} has_turn_id={}",
                role,
                path.display(),
                expected_version,
                actual_version,
                has_turn_id
            );
            if actual_version != expected_version {
                crate::log_debug!(
                    "runtime_layout: bridge version mismatch role={} expected={} actual={}",
                    role,
                    expected_version,
                    actual_version
                );
            }
            if expect_turn_id && !has_turn_id {
                crate::log_debug!(
                    "runtime_layout: bridge missing turn_id forwarding role={} path={}",
                    role,
                    path.display()
                );
            }
        }
        Err(err) => {
            crate::log_debug!(
                "runtime_layout: bridge status read failed role={} path={} err={}",
                role,
                path.display(),
                err
            );
        }
    }
}

fn bridge_version(content: &str) -> Option<&str> {
    content
        .lines()
        .find_map(|line| line.strip_prefix(BRIDGE_VERSION_PREFIX))
        .map(str::trim)
}
