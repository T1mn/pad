use crate::app::App;
use crate::theme::{OpenCodeModelConfig, ProviderConfig};

pub(super) fn selected_provider_models_len(app: &App) -> Option<usize> {
    app.config
        .agents
        .get(app.relay_selected_agent)?
        .providers
        .get(app.relay_selected_provider)
        .map(|provider| provider.models.len())
}

pub(super) fn selected_provider_model(app: &App) -> Option<&OpenCodeModelConfig> {
    app.config
        .agents
        .get(app.relay_selected_agent)?
        .providers
        .get(app.relay_selected_provider)?
        .models
        .get(app.relay_popup_selected)
}

pub(super) fn unique_model_id(
    provider: &ProviderConfig,
    raw: &str,
    skip_idx: Option<usize>,
) -> String {
    let base = raw.trim();
    let base = if base.is_empty() { "model-1" } else { base };
    let mut candidate = base.to_string();
    let mut suffix = 2usize;
    loop {
        let conflict = provider.models.iter().enumerate().any(|(idx, model)| {
            if Some(idx) == skip_idx {
                return false;
            }
            model.id == candidate
        });
        if !conflict {
            return candidate;
        }
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
}
