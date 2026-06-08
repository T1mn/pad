use crate::theme::{AgentConfig, ProviderConfig};

pub(super) fn relay_agent_matches(left: &AgentConfig, right: &AgentConfig) -> bool {
    left.active_provider == right.active_provider
        && left.default_model == right.default_model
        && left.small_model == right.small_model
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

pub(super) fn relay_reload_applied_title(locale: crate::i18n::Locale) -> &'static str {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        "Relay 已更新"
    } else {
        "Relay reloaded"
    }
}

pub(super) fn relay_reload_applied_body(
    locale: crate::i18n::Locale,
    path: &std::path::Path,
) -> String {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        format!("已应用 {}", path.display())
    } else {
        format!("Applied {}", path.display())
    }
}

pub(super) fn relay_reload_deferred_title(locale: crate::i18n::Locale) -> &'static str {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        "Relay 变更已暂存"
    } else {
        "Relay reload deferred"
    }
}

pub(super) fn relay_reload_deferred_body(
    locale: crate::i18n::Locale,
    path: &std::path::Path,
) -> String {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        format!("结束编辑后应用 {}", path.display())
    } else {
        format!("Will apply after editing {}", path.display())
    }
}
