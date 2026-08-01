mod agents;
mod backup;
mod codex;
mod sections;

use super::*;

/// 一次配置读取的结果。调用方必须能区分"读到了用户配置"和"文件坏了，这次是默认值"，
/// 否则下一次 `Config::save()` 会把 default 写回去，provider / api_key / bot_token 全丢。
pub struct ConfigLoadReport {
    pub config: Config,
    pub recovery: Option<ConfigRecovery>,
}

/// 解析失败时的现场信息：原始错误 + 损坏文件的备份路径。
pub struct ConfigRecovery {
    pub source: PathBuf,
    pub error: String,
    pub backup: Option<PathBuf>,
}

impl ConfigRecovery {
    pub fn describe(&self) -> String {
        match &self.backup {
            Some(backup) => format!("{}\nbackup: {}", self.error, backup.display()),
            None => format!(
                "{}\n(backup failed, {} left as is)",
                self.error,
                self.source.display()
            ),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        Self::load_reported().config
    }

    pub fn load_reported() -> ConfigLoadReport {
        let Some(load_path) = Self::resolved_config_path() else {
            return ConfigLoadReport {
                config: Self::default(),
                recovery: None,
            };
        };
        match Self::load_from_path(&load_path) {
            Ok(config) => ConfigLoadReport {
                config,
                recovery: None,
            },
            Err(error) => {
                let backup = backup::preserve_broken_config(&load_path);
                let recovery = ConfigRecovery {
                    source: load_path,
                    error,
                    backup,
                };
                crate::log_debug!("config: falling back to defaults: {}", recovery.describe());
                ConfigLoadReport {
                    config: Self::default(),
                    recovery: Some(recovery),
                }
            }
        }
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
