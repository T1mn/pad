use super::super::super::super::common::mask_secret_prefix;
use crate::app::App;
use crate::theme::{AgentConfig, ProviderConfig};

pub(super) fn masked_api_key(app: &App, agent: &AgentConfig, provider: &ProviderConfig) -> String {
    if app.relay_editing && app.relay_edit_field == 2 {
        format!("{}|", app.relay_edit_buffer)
    } else if agent.name == "codex" {
        mask_secret_prefix(
            provider.codex_auth_token().as_deref().unwrap_or_default(),
            10,
        )
    } else {
        mask_secret_prefix(&provider.api_key, 12)
    }
}
