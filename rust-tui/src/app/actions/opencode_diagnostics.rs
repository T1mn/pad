mod collect;
mod report;
mod text;

use super::*;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

impl App {
    pub fn export_opencode_diagnostics(&mut self) -> bool {
        match export_diagnostics(&opencode_cli::opencode_command(&self.config)) {
            Ok(path) => {
                self.show_action_toast(
                    text::diagnostics_saved_title(self.locale),
                    &path.display().to_string(),
                );
                true
            }
            Err(err) => {
                self.show_action_toast(
                    text::diagnostics_failed_title(self.locale),
                    &err.to_string(),
                );
                false
            }
        }
    }
}

fn export_diagnostics(command: &OsString) -> io::Result<PathBuf> {
    let sections = collect::collect_diagnostics_sections(command);
    let path = report::diagnostics_path(
        crate::paths::opencode_diagnostics_dir().as_path(),
        report::current_unix_secs(),
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    report::write_private_report(&path, &report::format_report(&sections))?;
    Ok(path)
}

#[cfg(test)]
pub(in crate::app::actions) use collect::DiagnosticsSection;
#[cfg(test)]
pub(in crate::app::actions) use report::{diagnostics_path, format_report};

#[cfg(test)]
#[path = "opencode_diagnostics_tests.rs"]
mod opencode_diagnostics_tests;
