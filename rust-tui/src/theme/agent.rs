use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenCodeModelConfig {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub name: String,
    pub cmd: String,
    pub providers: Vec<ProviderConfig>,
    pub active_provider: Option<usize>,
    pub default_model: String,
    pub small_model: String,
    #[allow(dead_code)]
    pub base_url: Option<String>,
    #[allow(dead_code)]
    pub api_key: Option<String>,
}

impl AgentConfig {
    pub fn active(&self) -> Option<&ProviderConfig> {
        self.active_provider.and_then(|i| self.providers.get(i))
    }

    pub fn opencode_model_options(&self) -> Vec<(String, String)> {
        let mut options = Vec::new();
        for provider in &self.providers {
            let provider_key = provider.opencode_provider_key();
            let provider_label = if provider.label.trim().is_empty() {
                provider_key
            } else {
                provider.label.as_str()
            };
            for model in &provider.models {
                if model.id.trim().is_empty() {
                    continue;
                }
                let value = format!("{provider_key}/{}", model.id.trim());
                let model_name = if model.name.trim().is_empty() {
                    model.id.trim()
                } else {
                    model.name.trim()
                };
                let label = format!("{provider_label} / {model_name} ({})", model.id.trim());
                options.push((value, label));
            }
        }
        options
    }

    pub fn opencode_first_model_value(&self) -> Option<String> {
        self.opencode_model_options()
            .into_iter()
            .next()
            .map(|(value, _)| value)
    }

    pub fn repair_opencode_model_refs(&mut self) {
        let valid: std::collections::HashSet<String> = self
            .opencode_model_options()
            .into_iter()
            .map(|(value, _)| value)
            .collect();

        if !self.default_model.is_empty() && !valid.contains(&self.default_model) {
            self.default_model = self.opencode_first_model_value().unwrap_or_default();
        }
        if !self.small_model.is_empty() && !valid.contains(&self.small_model) {
            self.small_model.clear();
        }
    }

    pub fn rename_opencode_provider_key(&mut self, old_key: &str, new_key: &str) {
        if old_key == new_key {
            return;
        }
        if self.default_model.starts_with(&format!("{old_key}/")) {
            self.default_model = self.default_model.replacen(old_key, new_key, 1);
        }
        if self.small_model.starts_with(&format!("{old_key}/")) {
            self.small_model = self.small_model.replacen(old_key, new_key, 1);
        }
    }

    pub fn rename_opencode_model_id(&mut self, provider_key: &str, old_id: &str, new_id: &str) {
        let old_value = format!("{provider_key}/{old_id}");
        let new_value = format!("{provider_key}/{new_id}");
        if self.default_model == old_value {
            self.default_model = new_value.clone();
        }
        if self.small_model == old_value {
            self.small_model = new_value;
        }
    }
}
