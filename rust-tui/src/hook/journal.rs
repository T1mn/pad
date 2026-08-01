use super::HookEvent;
use crate::log_debug;
use std::fs::OpenOptions;
use std::io::Write;

pub(super) fn append_hook_event_journal(event: &HookEvent) {
    let path = crate::paths::hook_events_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(crate::atomic_file::PRIVATE_MODE);
    }
    match options.open(&path) {
        Ok(mut file) => {
            if let Err(err) = crate::atomic_file::ensure_private(&path) {
                log_debug!("hook_listener: failed to protect hook journal: {}", err);
                return;
            }
            if let Ok(line) = serde_json::to_string(event) {
                let _ = writeln!(file, "{}", line);
            }
        }
        Err(err) => {
            log_debug!("hook_listener: failed to append hook journal: {}", err);
        }
    }
}
