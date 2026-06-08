use crate::app::App;
use crate::relay;

pub(in crate::event::modes::relay_settings) fn persist_relay_config(
    app: &mut App,
    agent_idx: usize,
) {
    if let Some(agent) = app.config.agents.get_mut(agent_idx) {
        if agent.name == "opencode" {
            agent.repair_opencode_model_refs();
        }
    }
    app.config.save();
    relay::apply_runtime_configs(
        &app.config.agents,
        &app.config.agent_permissions,
        &app.config.codex,
    );
}
