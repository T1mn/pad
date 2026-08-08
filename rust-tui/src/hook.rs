mod journal {
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
}
mod listener;
mod model {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HookTmuxInfo {
        pub pane_id: Option<String>,
        pub session_name: Option<String>,
        pub window_index: Option<String>,
        pub pane_index: Option<String>,
        pub pane_current_path: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HookEvent {
        pub event: String,
        #[serde(default)]
        pub turn_id: Option<String>,
        pub session_id: Option<String>,
        pub transcript_path: Option<String>,
        pub cwd: Option<String>,
        pub prompt: Option<String>,
        pub last_assistant_message: Option<String>,
        pub timestamp: Option<String>,
        pub tmux: HookTmuxInfo,
    }
}

pub use listener::{hook_socket_is_active, start_hook_listener};
pub use model::HookEvent;
#[cfg(test)]
pub use model::HookTmuxInfo;
