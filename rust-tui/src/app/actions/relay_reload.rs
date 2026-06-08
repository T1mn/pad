mod apply;
mod source;

use super::*;
use std::time::Duration;

#[path = "relay_reload_helpers.rs"]
mod relay_reload_helpers;
use relay_reload_helpers::{relay_reload_applied_body, relay_reload_applied_title};

const RELAY_CONFIG_POLL_INTERVAL: Duration = Duration::from_secs(1);

impl App {
    pub fn poll_external_relay_config_if_due(&mut self) {
        if self.relay_config_last_poll_at.elapsed() < RELAY_CONFIG_POLL_INTERVAL {
            return;
        }
        self.relay_config_last_poll_at = std::time::Instant::now();

        if !self.refresh_relay_config_source_state() {
            return;
        }

        self.try_apply_external_relay_config(/*forced*/ false);
    }

    pub fn apply_pending_external_relay_reload_if_ready(&mut self) {
        if !self.pending_external_relay_reload || self.is_relay_reload_deferred() {
            return;
        }

        self.pending_external_relay_reload = false;
        self.refresh_relay_config_source_state();
        self.try_apply_external_relay_config(/*forced*/ true);
    }

    fn is_relay_reload_deferred(&self) -> bool {
        self.relay_editing || self.relay_popup_editing
    }

    fn try_apply_external_relay_config(&mut self, forced: bool) {
        let Some(path) = self.relay_config_source_path.clone() else {
            return;
        };

        let loaded = match crate::theme::Config::load_from_path(&path) {
            Ok(config) => config,
            Err(err) => {
                crate::log_debug!(
                    "relay.reload: ignore invalid external config path={} err={}",
                    path.display(),
                    err
                );
                return;
            }
        };

        if !self.external_relay_config_differs(&loaded) {
            return;
        }

        if !forced && self.is_relay_reload_deferred() {
            self.defer_external_relay_reload(&path);
            return;
        }

        self.apply_external_relay_config(loaded);
        let toast_body = relay_reload_applied_body(self.locale, &path);
        self.show_action_toast(relay_reload_applied_title(self.locale), &toast_body);
        crate::log_debug!(
            "relay.reload: applied path={} forced={}",
            path.display(),
            forced
        );
    }
}

#[cfg(test)]
#[path = "relay_reload_tests.rs"]
mod relay_reload_tests;
