use std::ffi::OsString;
use std::io;
use std::process::Command;

pub(in crate::app::actions) struct DiagnosticsSection {
    pub(in crate::app::actions) title: &'static str,
    pub(in crate::app::actions) body: String,
}

pub(super) fn collect_diagnostics_sections(command: &OsString) -> Vec<DiagnosticsSection> {
    [
        diagnostics_section(command, "version", &["--version"]),
        diagnostics_section(command, "db path", &["db", "path"]),
        diagnostics_section(command, "debug info", &["debug", "info"]),
        diagnostics_section(command, "debug paths", &["debug", "paths"]),
        diagnostics_section(command, "debug config", &["debug", "config"]),
        diagnostics_section(command, "providers list", &["providers", "list"]),
        diagnostics_section(command, "models --verbose", &["models", "--verbose"]),
        diagnostics_section(command, "agent list", &["agent", "list"]),
        diagnostics_section(command, "mcp list", &["mcp", "list"]),
    ]
    .into_iter()
    .collect()
}

fn diagnostics_section(
    command: &OsString,
    title: &'static str,
    args: &[&str],
) -> DiagnosticsSection {
    let body = match run_opencode(command, args) {
        Ok(output) => output,
        Err(err) => format!("ERROR: {err}"),
    };
    DiagnosticsSection { title, body }
}

fn run_opencode(command: &OsString, args: &[&str]) -> io::Result<String> {
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(if stderr.is_empty() {
            format!(
                "opencode {} exited with {}",
                format_opencode_args(args),
                output.status
            )
        } else {
            stderr
        }));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        Err(io::Error::other(format!(
            "opencode {} returned empty output",
            format_opencode_args(args)
        )))
    } else {
        Ok(stdout)
    }
}

fn format_opencode_args(args: &[&str]) -> String {
    let mut formatted = String::new();
    for arg in args {
        if !formatted.is_empty() {
            formatted.push(' ');
        }
        formatted.push_str(arg);
    }
    formatted
}

#[cfg(test)]
#[path = "collect_tests.rs"]
mod tests;
