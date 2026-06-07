use super::*;
use crate::app::state::RelayView;
use crate::relay;
use crate::theme::Config;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[path = "relay_reload_helpers.rs"]
mod relay_reload_helpers;
use relay_reload_helpers::{
    relay_agent_matches, relay_reload_applied_body, relay_reload_applied_title,
    relay_reload_deferred_body, relay_reload_deferred_title,
};

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

    fn refresh_relay_config_source_state(&mut self) -> bool {
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

    fn try_apply_external_relay_config(&mut self, forced: bool) {
        let Some(path) = self.relay_config_source_path.clone() else {
            return;
        };

        let loaded = match Config::load_from_path(&path) {
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
            self.pending_external_relay_reload = true;
            let toast_body = relay_reload_deferred_body(self.locale, &path);
            self.show_action_toast(relay_reload_deferred_title(self.locale), &toast_body);
            crate::log_debug!(
                "relay.reload: deferred path={} reason=editing",
                path.display()
            );
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

    fn external_relay_config_differs(&self, loaded: &Config) -> bool {
        self.config.agents.iter().any(|current| {
            loaded
                .agents
                .iter()
                .find(|candidate| candidate.name == current.name)
                .is_some_and(|candidate| !relay_agent_matches(current, candidate))
        })
    }

    fn apply_external_relay_config(&mut self, loaded: Config) {
        for current in &mut self.config.agents {
            let Some(source) = loaded
                .agents
                .iter()
                .find(|candidate| candidate.name == current.name)
            else {
                continue;
            };

            current.providers = source.providers.clone();
            current.active_provider = source.active_provider;
            current.default_model = source.default_model.clone();
            current.small_model = source.small_model.clone();
            current.base_url = source.base_url.clone();
            current.api_key = source.api_key.clone();

            if current.name == "opencode" {
                current.repair_opencode_model_refs();
            }
        }

        self.normalize_relay_ui_after_external_reload();
        relay::apply_runtime_configs(
            &self.config.agents,
            &self.config.agent_permissions,
            &self.config.codex,
        );
        self.dirty = true;
    }

    fn normalize_relay_ui_after_external_reload(&mut self) {
        if self.config.agents.is_empty() {
            self.relay_selected_agent = 0;
            self.relay_selected_provider = 0;
            self.relay_view = RelayView::AgentList;
            self.relay_edit_field = 0;
            self.relay_editing = false;
            self.relay_edit_buffer.clear();
            self.clear_relay_popup_state();
            return;
        }

        self.relay_selected_agent = self
            .relay_selected_agent
            .min(self.config.agents.len().saturating_sub(1));
        self.relay_edit_buffer.clear();
        self.clear_relay_popup_state();

        let selected_agent = &self.config.agents[self.relay_selected_agent];
        if selected_agent.providers.is_empty() {
            self.relay_selected_provider = 0;
            if self.relay_view == RelayView::DetailPane {
                self.relay_view = RelayView::ProviderList;
            }
            self.relay_edit_field = 0;
            return;
        }

        self.relay_selected_provider = self
            .relay_selected_provider
            .min(selected_agent.providers.len().saturating_sub(1));
        let max_fields = if selected_agent.name == "opencode" {
            6
        } else {
            3
        };
        if self.relay_edit_field >= max_fields {
            self.relay_edit_field = max_fields.saturating_sub(1);
        }
    }
}

#[cfg(test)]
#[path = "relay_reload_tests.rs"]
mod relay_reload_tests;
