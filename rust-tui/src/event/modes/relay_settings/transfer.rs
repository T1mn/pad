use super::edit::persist_relay_config;
use super::relay_field_count;
use crate::app::state::RelayView;
use crate::app::App;
use crate::relay;

pub(super) fn export_selected_codex_provider(app: &mut App) {
    let Some(agent) = app.config.agents.get(app.relay_selected_agent) else {
        return;
    };

    match relay::write_codex_relay_export(agent) {
        Ok(path) => {
            let body = codex_export_saved_body(app.locale, &path);
            app.show_action_toast(codex_export_saved_title(app.locale), &body);
        }
        Err(err) => {
            app.show_action_toast(codex_export_failed_title(app.locale), &err.to_string());
        }
    }
}

pub(super) fn import_selected_codex_provider(app: &mut App) {
    match relay::read_codex_relay_import() {
        Ok((providers, active_provider, path)) => {
            let agent_idx = app.relay_selected_agent;
            if let Some(agent) = app.config.agents.get_mut(agent_idx) {
                agent.providers = providers;
                agent.active_provider = active_provider;
            }
            normalize_codex_relay_selection(app);
            persist_relay_config(app, agent_idx);
            let body = codex_import_saved_body(app.locale, &path);
            app.show_action_toast(codex_import_saved_title(app.locale), &body);
        }
        Err(err) => {
            app.show_action_toast(codex_import_failed_title(app.locale), &err);
        }
    }
}

fn normalize_codex_relay_selection(app: &mut App) {
    let Some(agent) = app.config.agents.get(app.relay_selected_agent) else {
        app.relay_selected_provider = 0;
        return;
    };

    if agent.providers.is_empty() {
        app.relay_selected_provider = 0;
        app.relay_view = RelayView::ProviderList;
        app.relay_edit_field = 0;
        return;
    }

    app.relay_selected_provider = agent
        .active_provider
        .unwrap_or(app.relay_selected_provider)
        .min(agent.providers.len().saturating_sub(1));
    app.relay_edit_field = app
        .relay_edit_field
        .min(relay_field_count(app).saturating_sub(1));
}

fn codex_export_saved_title(locale: crate::i18n::Locale) -> &'static str {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        "Codex relay 已导出"
    } else {
        "Codex relay exported"
    }
}

fn codex_export_failed_title(locale: crate::i18n::Locale) -> &'static str {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        "Codex relay 导出失败"
    } else {
        "Codex relay export failed"
    }
}

fn codex_export_saved_body(locale: crate::i18n::Locale, path: &std::path::Path) -> String {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        format!("已写入 {}", path.display())
    } else {
        format!("Wrote {}", path.display())
    }
}

fn codex_import_saved_title(locale: crate::i18n::Locale) -> &'static str {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        "Codex relay 已导入"
    } else {
        "Codex relay imported"
    }
}

fn codex_import_failed_title(locale: crate::i18n::Locale) -> &'static str {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        "Codex relay 导入失败"
    } else {
        "Codex relay import failed"
    }
}

fn codex_import_saved_body(locale: crate::i18n::Locale, path: &std::path::Path) -> String {
    if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW | crate::i18n::Locale::Ja
    ) {
        format!("已从 {} 导入", path.display())
    } else {
        format!("Imported from {}", path.display())
    }
}
