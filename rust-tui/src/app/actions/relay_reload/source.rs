use super::*;
use crate::theme::Config;
use std::fs;
use std::path::{Path, PathBuf};

use super::relay_reload_helpers::{relay_reload_deferred_body, relay_reload_deferred_title};

impl App {
    pub(super) fn refresh_relay_config_source_state(&mut self) -> bool {
        let (path, modified_ms, len) = Self::capture_relay_config_source_state();
        let changed = self.relay_config_source_path != path
            || self.relay_config_source_modified_ms != modified_ms
            || self.relay_config_source_len != len;
        self.relay_config_source_path = path;
        self.relay_config_source_modified_ms = modified_ms;
        self.relay_config_source_len = len;
        changed
    }

    fn capture_relay_config_source_state() -> (Option<PathBuf>, Option<u128>, Option<u64>) {
        let Some(path) = Config::resolved_config_path() else {
            return (None, None, None);
        };

        let metadata = fs::metadata(&path).ok();
        let modified_ms = metadata
            .as_ref()
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis());
        let len = metadata.as_ref().map(|meta| meta.len());

        (Some(path), modified_ms, len)
    }

    pub(super) fn defer_external_relay_reload(&mut self, path: &Path) {
        self.pending_external_relay_reload = true;
        let toast_body = relay_reload_deferred_body(self.locale, path);
        self.show_action_toast(relay_reload_deferred_title(self.locale), &toast_body);
        crate::log_debug!(
            "relay.reload: deferred path={} reason=editing",
            path.display()
        );
    }
}
