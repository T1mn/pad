use super::HookEvent;
use crate::log_debug;
use std::fs::OpenOptions;
use std::io::Write;

pub(super) fn append_hook_event_journal(event: &HookEvent) {
    let path = crate::paths::hook_events_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            if let Ok(line) = serde_json::to_string(event) {
                let _ = writeln!(file, "{}", line);
            }
        }
        Err(err) => {
            log_debug!("hook_listener: failed to append hook journal: {}", err);
        }
    }
}
