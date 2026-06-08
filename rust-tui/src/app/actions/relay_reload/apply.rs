use super::*;
use crate::app::state::RelayView;
use crate::relay;
use crate::theme::Config;

use super::relay_reload_helpers::relay_agent_matches;

impl App {
    pub(super) fn external_relay_config_differs(&self, loaded: &Config) -> bool {
        self.config.agents.iter().any(|current| {
            loaded
                .agents
                .iter()
                .find(|candidate| candidate.name == current.name)
                .is_some_and(|candidate| !relay_agent_matches(current, candidate))
        })
    }

    pub(super) fn apply_external_relay_config(&mut self, loaded: Config) {
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
