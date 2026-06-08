mod agents;
mod codex;
mod sections;

use super::*;

impl Config {
    pub fn load() -> Self {
        let Some(load_path) = Self::resolved_config_path() else {
            return Self::default();
        };
        Self::load_from_path(&load_path).unwrap_or_default()
    }

    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| format!("read {} failed: {err}", path.display()))?;
        let table: HashMap<String, toml::Value> = toml::from_str(&content)
            .map_err(|err| format!("parse {} failed: {err}", path.display()))?;

        let mut config = Self::default();
        sections::apply_root_fields(&table, &mut config);
        sections::apply_section_fields(&table, &mut config);
        agents::apply_agents(&table, &mut config);
        Ok(config)
    }
}
