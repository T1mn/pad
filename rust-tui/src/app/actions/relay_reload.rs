use super::*;
use crate::app::state::RelayView;
use crate::relay;
use crate::theme::{AgentConfig, Config, ProviderConfig};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

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

fn relay_agent_matches(left: &AgentConfig, right: &AgentConfig) -> bool {
    left.active_provider == right.active_provider
        && left.default_model == right.default_model
        && left.small_model == right.small_model
        && left.base_url == right.base_url
        && left.api_key == right.api_key
        && left.providers.len() == right.providers.len()
        && left
            .providers
            .iter()
            .zip(&right.providers)
            .all(|(current, candidate)| relay_provider_matches(current, candidate))
}

fn relay_provider_matches(left: &ProviderConfig, right: &ProviderConfig) -> bool {
    left.label == right.label
        && left.base_url == right.base_url
        && left.api_key == right.api_key
        && left.env_key == right.env_key
        && left.wire_api == right.wire_api
        && left.provider_key == right.provider_key
        && left.npm_package == right.npm_package
        && left.models == right.models
}

fn relay_reload_applied_title(locale: crate::i18n::Locale) -> &'static str {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        "Relay 已更新"
    } else {
        "Relay reloaded"
    }
}

fn relay_reload_applied_body(locale: crate::i18n::Locale, path: &std::path::Path) -> String {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        format!("已应用 {}", path.display())
    } else {
        format!("Applied {}", path.display())
    }
}

fn relay_reload_deferred_title(locale: crate::i18n::Locale) -> &'static str {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        "Relay 变更已暂存"
    } else {
        "Relay reload deferred"
    }
}

fn relay_reload_deferred_body(locale: crate::i18n::Locale, path: &std::path::Path) -> String {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        format!("结束编辑后应用 {}", path.display())
    } else {
        format!("Will apply after editing {}", path.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ProviderConfig;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
        let _guard = crate::test_support::home_env_lock()
            .lock()
            .expect("lock relay reload tests");
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let home = std::env::temp_dir().join(format!("pad-relay-reload-{name}-{stamp}"));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("create temp home");

        let prev_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &home);

        let result = f();

        if let Some(prev) = prev_home {
            std::env::set_var("HOME", prev);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = std::fs::remove_dir_all(&home);
        result
    }

    fn sample_provider(label: &str, base_url: &str, api_key: &str) -> ProviderConfig {
        ProviderConfig {
            label: label.to_string(),
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            env_key: String::new(),
            wire_api: "responses".to_string(),
            provider_key: crate::theme::normalize_provider_key(label),
            npm_package: "@ai-sdk/openai-compatible".to_string(),
            models: Vec::new(),
            test_status: Some(true),
            test_http_status: Some(200),
            test_latency_ms: Some(12),
            test_result: Some("ok".to_string()),
        }
    }

    #[test]
    fn external_relay_reload_applies_immediately_when_not_editing() {
        with_temp_home("immediate", || {
            let mut config = Config::default();
            let codex = config
                .agents
                .iter_mut()
                .find(|agent| agent.name == "codex")
                .expect("codex agent");
            codex.providers = vec![sample_provider("a", "https://a.example/v1", "sk-a")];
            codex.active_provider = Some(0);
            config.save();

            let mut app = App::new();
            assert_eq!(app.config.agents[1].providers[0].label, "a");

            let mut updated = Config::load();
            let codex = updated
                .agents
                .iter_mut()
                .find(|agent| agent.name == "codex")
                .expect("updated codex agent");
            codex.providers = vec![sample_provider("b", "https://b.example/v1", "sk-b")];
            codex.active_provider = Some(0);
            updated.save();

            app.relay_config_last_poll_at -= RELAY_CONFIG_POLL_INTERVAL;
            app.poll_external_relay_config_if_due();

            let codex = app
                .config
                .agents
                .iter()
                .find(|agent| agent.name == "codex")
                .expect("reloaded codex agent");
            assert_eq!(codex.providers[0].label, "b");
            assert_eq!(codex.providers[0].base_url, "https://b.example/v1");
            assert_eq!(codex.providers[0].api_key, "sk-b");
            assert!(!app.pending_external_relay_reload);
            assert_eq!(
                app.preview
                    .copy_toast
                    .as_ref()
                    .map(|toast| toast.title.as_str()),
                Some("Relay reloaded")
            );
        });
    }

    #[test]
    fn external_relay_reload_is_deferred_while_editing() {
        with_temp_home("deferred", || {
            let mut config = Config::default();
            let codex = config
                .agents
                .iter_mut()
                .find(|agent| agent.name == "codex")
                .expect("codex agent");
            codex.providers = vec![sample_provider("a", "https://a.example/v1", "sk-a")];
            codex.active_provider = Some(0);
            config.save();

            let mut app = App::new();
            app.relay_editing = true;

            let mut updated = Config::load();
            let codex = updated
                .agents
                .iter_mut()
                .find(|agent| agent.name == "codex")
                .expect("updated codex agent");
            codex.providers = vec![sample_provider("b", "https://b.example/v1", "sk-b")];
            codex.active_provider = Some(0);
            updated.save();

            app.relay_config_last_poll_at -= RELAY_CONFIG_POLL_INTERVAL;
            app.poll_external_relay_config_if_due();

            let codex = app
                .config
                .agents
                .iter()
                .find(|agent| agent.name == "codex")
                .expect("current codex agent");
            assert_eq!(codex.providers[0].label, "a");
            assert!(app.pending_external_relay_reload);
            assert_eq!(
                app.preview
                    .copy_toast
                    .as_ref()
                    .map(|toast| toast.title.as_str()),
                Some("Relay reload deferred")
            );

            app.relay_editing = false;
            app.apply_pending_external_relay_reload_if_ready();

            let codex = app
                .config
                .agents
                .iter()
                .find(|agent| agent.name == "codex")
                .expect("reloaded codex agent");
            assert_eq!(codex.providers[0].label, "b");
            assert!(!app.pending_external_relay_reload);
            assert_eq!(
                app.preview
                    .copy_toast
                    .as_ref()
                    .map(|toast| toast.title.as_str()),
                Some("Relay reloaded")
            );
        });
    }

    #[test]
    fn invalid_external_relay_config_is_ignored() {
        with_temp_home("invalid", || {
            let mut config = Config::default();
            let codex = config
                .agents
                .iter_mut()
                .find(|agent| agent.name == "codex")
                .expect("codex agent");
            codex.providers = vec![sample_provider("a", "https://a.example/v1", "sk-a")];
            codex.active_provider = Some(0);
            config.save();

            let mut app = App::new();
            std::fs::write(Config::config_path(), "[[agents]\n").expect("write invalid config");

            app.relay_config_last_poll_at -= RELAY_CONFIG_POLL_INTERVAL;
            app.poll_external_relay_config_if_due();

            let codex = app
                .config
                .agents
                .iter()
                .find(|agent| agent.name == "codex")
                .expect("current codex agent");
            assert_eq!(codex.providers[0].label, "a");
            assert!(!app.pending_external_relay_reload);
        });
    }

    #[test]
    fn external_relay_reload_clamps_provider_selection() {
        with_temp_home("clamp", || {
            let mut config = Config::default();
            let codex = config
                .agents
                .iter_mut()
                .find(|agent| agent.name == "codex")
                .expect("codex agent");
            codex.providers = vec![
                sample_provider("a", "https://a.example/v1", "sk-a"),
                sample_provider("b", "https://b.example/v1", "sk-b"),
            ];
            codex.active_provider = Some(1);
            config.save();

            let mut app = App::new();
            app.relay_selected_agent = app
                .config
                .agents
                .iter()
                .position(|agent| agent.name == "codex")
                .expect("codex index");
            app.relay_selected_provider = 1;
            app.relay_view = RelayView::DetailPane;
            app.relay_edit_field = 2;

            let mut updated = Config::load();
            let codex = updated
                .agents
                .iter_mut()
                .find(|agent| agent.name == "codex")
                .expect("updated codex agent");
            codex.providers = vec![sample_provider(
                "only",
                "https://only.example/v1",
                "sk-only",
            )];
            codex.active_provider = Some(0);
            updated.save();

            app.relay_config_last_poll_at -= RELAY_CONFIG_POLL_INTERVAL;
            app.poll_external_relay_config_if_due();

            assert_eq!(app.relay_selected_provider, 0);
            assert!(matches!(app.relay_view, RelayView::DetailPane));
            assert_eq!(app.relay_edit_field, 2);

            let codex = app
                .config
                .agents
                .iter()
                .find(|agent| agent.name == "codex")
                .expect("reloaded codex agent");
            assert_eq!(codex.providers.len(), 1);
            assert_eq!(codex.providers[0].label, "only");
        });
    }
}
