use crate::app::state::{Mode, RelayPopupMode, RelayView};
use crate::app::App;
use crate::relay;
use crate::theme::{normalize_provider_key, OpenCodeModelConfig, ProviderConfig};
use crossterm::event::KeyCode;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelayHost {
    Standalone,
    Settings,
}

pub(crate) fn handle_relay_settings_mode(app: &mut App, key: KeyCode) {
    let _ = handle_relay_key(app, key, RelayHost::Standalone);
}

pub(crate) fn handle_relay_key(app: &mut App, key: KeyCode, host: RelayHost) -> bool {
    if app.relay_popup_mode != RelayPopupMode::None {
        return handle_relay_popup_key(app, key);
    }

    if app.relay_editing {
        return handle_relay_field_edit(app, key);
    }

    match app.relay_view {
        RelayView::AgentList => handle_agent_list_key(app, key, host),
        RelayView::ProviderList => handle_provider_list_key(app, key, host),
        RelayView::DetailPane => handle_detail_pane_key(app, key, host),
    }
}

fn handle_agent_list_key(app: &mut App, key: KeyCode, host: RelayHost) -> bool {
    match key {
        KeyCode::Esc => {
            exit_relay(app, host);
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.config.agents.len().saturating_sub(1);
            if app.relay_selected_agent < max {
                app.relay_selected_agent += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.relay_selected_agent > 0 {
                app.relay_selected_agent -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            app.relay_view = RelayView::ProviderList;
            let active = app
                .config
                .agents
                .get(app.relay_selected_agent)
                .and_then(|agent| agent.active_provider)
                .unwrap_or(0);
            app.relay_selected_provider = active;
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_provider_list_key(app: &mut App, key: KeyCode, host: RelayHost) -> bool {
    match key {
        KeyCode::Esc => {
            if host == RelayHost::Settings {
                exit_relay(app, host);
            } else {
                app.relay_view = RelayView::AgentList;
            }
            app.dirty = true;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.relay_view = RelayView::AgentList;
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                let max = agent.providers.len().saturating_sub(1);
                if app.relay_selected_provider < max {
                    app.relay_selected_provider += 1;
                }
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.relay_selected_provider > 0 {
                app.relay_selected_provider -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
            if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                if !agent.providers.is_empty() {
                    app.relay_view = RelayView::DetailPane;
                    app.relay_edit_field = 0;
                    app.dirty = true;
                }
            }
        }
        KeyCode::Char(' ') => {
            let agent_idx = app.relay_selected_agent;
            let prov_idx = app.relay_selected_provider;
            if let Some(agent) = app.config.agents.get_mut(agent_idx) {
                if prov_idx < agent.providers.len() {
                    if agent.active_provider == Some(prov_idx) {
                        agent.active_provider = None;
                    } else {
                        agent.active_provider = Some(prov_idx);
                    }
                    persist_relay_config(app, agent_idx);
                }
            }
            app.dirty = true;
        }
        KeyCode::Char('a') => {
            add_provider(app);
            app.dirty = true;
        }
        KeyCode::Char('d') => {
            delete_provider(app);
            app.dirty = true;
        }
        KeyCode::Char('m') if selected_agent_name(app) == Some("opencode") => {
            app.relay_popup_mode = RelayPopupMode::OpenCodeDefaultModel;
            app.relay_popup_selected = selected_model_picker_index(app, false);
            app.dirty = true;
        }
        KeyCode::Char('M') if selected_agent_name(app) == Some("opencode") => {
            app.relay_popup_mode = RelayPopupMode::OpenCodeSmallModel;
            app.relay_popup_selected = selected_model_picker_index(app, true);
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_detail_pane_key(app: &mut App, key: KeyCode, host: RelayHost) -> bool {
    match key {
        KeyCode::Esc => {
            if host == RelayHost::Settings {
                exit_relay(app, host);
            } else {
                app.relay_view = RelayView::ProviderList;
            }
            app.dirty = true;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.relay_view = RelayView::ProviderList;
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let count = relay_field_count(app);
            app.relay_edit_field = (app.relay_edit_field + 1) % count;
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let count = relay_field_count(app);
            app.relay_edit_field = (app.relay_edit_field + count - 1) % count;
            app.dirty = true;
        }
        KeyCode::Enter => {
            if selected_agent_name(app) == Some("opencode") && app.relay_edit_field == 5 {
                app.relay_popup_mode = RelayPopupMode::OpenCodeModels;
                app.relay_popup_selected = 0;
                app.relay_popup_field = 0;
                app.relay_popup_editing = false;
                app.relay_popup_buffer.clear();
            } else if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                if let Some(prov) = agent.providers.get(app.relay_selected_provider) {
                    app.relay_edit_buffer = match agent.name.as_str() {
                        "opencode" => match app.relay_edit_field {
                            0 => prov.label.clone(),
                            1 => prov.provider_key.clone(),
                            2 => prov.npm_package.clone(),
                            3 => prov.base_url.clone(),
                            4 => prov.api_key.clone(),
                            _ => String::new(),
                        },
                        _ => match app.relay_edit_field {
                            0 => prov.label.clone(),
                            1 => prov.base_url.clone(),
                            2 => prov.api_key.clone(),
                            _ => String::new(),
                        },
                    };
                    app.relay_editing = true;
                }
            }
            app.dirty = true;
        }
        KeyCode::Char(' ') => {
            app.trigger_provider_test(app.relay_selected_agent, app.relay_selected_provider);
            app.dirty = true;
        }
        KeyCode::Char('y') | KeyCode::Char('Y') if selected_agent_name(app) == Some("codex") => {
            export_selected_codex_provider(app);
            app.dirty = true;
        }
        KeyCode::Char('i') | KeyCode::Char('I') if selected_agent_name(app) == Some("codex") => {
            import_selected_codex_provider(app);
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_relay_field_edit(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => {
            app.relay_editing = false;
            app.relay_edit_buffer.clear();
        }
        KeyCode::Enter => commit_relay_field_edit(app),
        KeyCode::Backspace => {
            app.relay_edit_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.relay_edit_buffer.push(c);
        }
        _ => {}
    }
    app.dirty = true;
    true
}

fn commit_relay_field_edit(app: &mut App) {
    let agent_idx = app.relay_selected_agent;
    let prov_idx = app.relay_selected_provider;
    let field = app.relay_edit_field;
    let value = app.relay_edit_buffer.trim().to_string();

    let prepared_provider_key = if field == 1 {
        app.config
            .agents
            .get(agent_idx)
            .map(|agent| uniquify_provider_key(agent, &value, Some(prov_idx)))
    } else {
        None
    };

    if let Some(agent) = app.config.agents.get_mut(agent_idx) {
        let mut provider_key_rename: Option<(String, String)> = None;
        let agent_name = agent.name.clone();
        if let Some(prov) = agent.providers.get_mut(prov_idx) {
            match agent_name.as_str() {
                "opencode" => match field {
                    0 => prov.label = value,
                    1 => {
                        let old_key = prov.provider_key.clone();
                        let new_key = prepared_provider_key.unwrap_or_else(|| old_key.clone());
                        prov.provider_key = new_key.clone();
                        provider_key_rename = Some((old_key, new_key));
                    }
                    2 => {
                        prov.npm_package = if value.is_empty() {
                            "@ai-sdk/openai-compatible".to_string()
                        } else {
                            value
                        };
                    }
                    3 => prov.base_url = value,
                    4 => prov.api_key = value,
                    _ => {}
                },
                "codex" => match field {
                    0 => prov.label = value,
                    1 => prov.base_url = value,
                    2 => {
                        prov.api_key = value;
                        prov.env_key.clear();
                    }
                    _ => {}
                },
                _ => match field {
                    0 => prov.label = value,
                    1 => prov.base_url = value,
                    2 => prov.api_key = value,
                    _ => {}
                },
            }
        }

        if let Some((old_key, new_key)) = provider_key_rename {
            agent.rename_opencode_provider_key(&old_key, &new_key);
        }
        if agent_name == "opencode" {
            agent.repair_opencode_model_refs();
        }
    }

    persist_relay_config(app, agent_idx);
    app.relay_editing = false;
    app.relay_edit_buffer.clear();
}

fn handle_relay_popup_key(app: &mut App, key: KeyCode) -> bool {
    if app.relay_popup_editing {
        return handle_relay_popup_edit(app, key);
    }

    match app.relay_popup_mode {
        RelayPopupMode::OpenCodeModels => handle_opencode_models_popup(app, key),
        RelayPopupMode::OpenCodeDefaultModel | RelayPopupMode::OpenCodeSmallModel => {
            handle_opencode_model_picker_popup(app, key)
        }
        RelayPopupMode::None => false,
    }
}

fn handle_relay_popup_edit(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => {
            app.relay_popup_editing = false;
            app.relay_popup_buffer.clear();
        }
        KeyCode::Enter => commit_opencode_model_field_edit(app),
        KeyCode::Backspace => {
            app.relay_popup_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.relay_popup_buffer.push(c);
        }
        _ => {}
    }
    app.dirty = true;
    true
}

fn handle_opencode_models_popup(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.clear_relay_popup_state();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(models_len) = selected_provider_models_len(app) {
                let max = models_len.saturating_sub(1);
                if app.relay_popup_selected < max {
                    app.relay_popup_selected += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up if app.relay_popup_selected > 0 => {
            app.relay_popup_selected -= 1;
        }
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
            app.relay_popup_field = (app.relay_popup_field + 1) % 2;
        }
        KeyCode::Enter => {
            open_opencode_model_field_edit(app);
        }
        KeyCode::Char('a') => {
            add_opencode_model(app);
        }
        KeyCode::Char('d') => {
            delete_opencode_model(app);
        }
        _ => {}
    }
    app.dirty = true;
    true
}

fn handle_opencode_model_picker_popup(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.clear_relay_popup_state();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = model_picker_options(app).len().saturating_sub(1);
            if app.relay_popup_selected < max {
                app.relay_popup_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up if app.relay_popup_selected > 0 => {
            app.relay_popup_selected -= 1;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let selected = model_picker_options(app)
                .get(app.relay_popup_selected)
                .map(|(value, _)| value.clone())
                .unwrap_or_default();
            let clear_small =
                app.relay_popup_mode == RelayPopupMode::OpenCodeSmallModel && selected.is_empty();
            if let Some(agent) = app.config.agents.get_mut(app.relay_selected_agent) {
                match app.relay_popup_mode {
                    RelayPopupMode::OpenCodeDefaultModel if !selected.is_empty() => {
                        agent.default_model = selected;
                    }
                    RelayPopupMode::OpenCodeSmallModel => {
                        if clear_small {
                            agent.small_model.clear();
                        } else {
                            agent.small_model = selected;
                        }
                    }
                    _ => {}
                }
                agent.repair_opencode_model_refs();
            }
            persist_relay_config(app, app.relay_selected_agent);
            app.clear_relay_popup_state();
        }
        _ => {}
    }
    app.dirty = true;
    true
}

fn open_opencode_model_field_edit(app: &mut App) {
    if let Some(model) = selected_provider_model(app) {
        app.relay_popup_buffer = if app.relay_popup_field == 0 {
            model.id.clone()
        } else {
            model.name.clone()
        };
        app.relay_popup_editing = true;
    }
}

fn commit_opencode_model_field_edit(app: &mut App) {
    let agent_idx = app.relay_selected_agent;
    let prov_idx = app.relay_selected_provider;
    let model_idx = app.relay_popup_selected;
    let field = app.relay_popup_field;
    let value = app.relay_popup_buffer.trim().to_string();

    let prepared_model_id = if field == 0 && !value.is_empty() {
        app.config
            .agents
            .get(agent_idx)
            .and_then(|agent| agent.providers.get(prov_idx))
            .map(|prov| unique_model_id(prov, &value, Some(model_idx)))
    } else {
        None
    };

    if let Some(agent) = app.config.agents.get_mut(agent_idx) {
        let provider_key = agent
            .providers
            .get(prov_idx)
            .map(|prov| prov.opencode_provider_key().to_string())
            .unwrap_or_default();
        let mut rename: Option<(String, String)> = None;

        if let Some(prov) = agent.providers.get_mut(prov_idx) {
            if field == 0 {
                if let Some(new_id) = prepared_model_id {
                    if let Some(model) = prov.models.get_mut(model_idx) {
                        let old_id = model.id.clone();
                        model.id = new_id.clone();
                        rename = Some((old_id, new_id));
                    }
                }
            } else if let Some(model) = prov.models.get_mut(model_idx) {
                model.name = value;
            }
        }

        if let Some((old_id, new_id)) = rename {
            agent.rename_opencode_model_id(&provider_key, &old_id, &new_id);
        }
        agent.repair_opencode_model_refs();
    }

    persist_relay_config(app, agent_idx);
    app.relay_popup_editing = false;
    app.relay_popup_buffer.clear();
}

fn add_provider(app: &mut App) {
    let agent_idx = app.relay_selected_agent;
    if let Some(agent) = app.config.agents.get_mut(agent_idx) {
        let label = format!("provider-{}", agent.providers.len() + 1);
        let provider_key = uniquify_provider_key(agent, &label, None);
        let mut provider = ProviderConfig {
            label,
            base_url: String::new(),
            api_key: String::new(),
            env_key: String::new(),
            wire_api: "responses".to_string(),
            provider_key,
            npm_package: "@ai-sdk/openai-compatible".to_string(),
            models: Vec::new(),
            test_status: None,
            test_http_status: None,
            test_latency_ms: None,
            test_result: None,
        };
        if agent.name == "opencode" {
            provider.models.push(OpenCodeModelConfig {
                id: "model-1".to_string(),
                name: "Model 1".to_string(),
            });
        }
        agent.providers.push(provider);
        app.relay_selected_provider = agent.providers.len().saturating_sub(1);
        if agent.name == "opencode" {
            agent.repair_opencode_model_refs();
        }
    }
    persist_relay_config(app, agent_idx);
}

fn delete_provider(app: &mut App) {
    let agent_idx = app.relay_selected_agent;
    let prov_idx = app.relay_selected_provider;
    if let Some(agent) = app.config.agents.get_mut(agent_idx) {
        if prov_idx < agent.providers.len() {
            agent.providers.remove(prov_idx);
            match agent.active_provider {
                Some(i) if i == prov_idx => agent.active_provider = None,
                Some(i) if i > prov_idx => agent.active_provider = Some(i - 1),
                _ => {}
            }
            if agent.name == "opencode" {
                agent.repair_opencode_model_refs();
            }
            if app.relay_selected_provider > 0
                && app.relay_selected_provider >= agent.providers.len()
            {
                app.relay_selected_provider = agent.providers.len().saturating_sub(1);
            }
        }
    }
    persist_relay_config(app, agent_idx);
}

fn add_opencode_model(app: &mut App) {
    let agent_idx = app.relay_selected_agent;
    let prov_idx = app.relay_selected_provider;
    if let Some(agent) = app.config.agents.get_mut(agent_idx) {
        if let Some(prov) = agent.providers.get_mut(prov_idx) {
            let model_id = unique_model_id(prov, "model-1", None);
            prov.models.push(OpenCodeModelConfig {
                id: model_id,
                name: "Model".to_string(),
            });
            app.relay_popup_selected = prov.models.len().saturating_sub(1);
            app.relay_popup_field = 0;
            agent.repair_opencode_model_refs();
        }
    }
    persist_relay_config(app, agent_idx);
    open_opencode_model_field_edit(app);
}

fn delete_opencode_model(app: &mut App) {
    let agent_idx = app.relay_selected_agent;
    let prov_idx = app.relay_selected_provider;
    let model_idx = app.relay_popup_selected;
    if let Some(agent) = app.config.agents.get_mut(agent_idx) {
        if let Some(prov) = agent.providers.get_mut(prov_idx) {
            if model_idx < prov.models.len() {
                prov.models.remove(model_idx);
                if app.relay_popup_selected > 0 && app.relay_popup_selected >= prov.models.len() {
                    app.relay_popup_selected = prov.models.len().saturating_sub(1);
                }
                agent.repair_opencode_model_refs();
            }
        }
    }
    persist_relay_config(app, agent_idx);
}

fn selected_provider_models_len(app: &App) -> Option<usize> {
    app.config
        .agents
        .get(app.relay_selected_agent)?
        .providers
        .get(app.relay_selected_provider)
        .map(|provider| provider.models.len())
}

fn selected_provider_model(app: &App) -> Option<&OpenCodeModelConfig> {
    app.config
        .agents
        .get(app.relay_selected_agent)?
        .providers
        .get(app.relay_selected_provider)?
        .models
        .get(app.relay_popup_selected)
}

fn persist_relay_config(app: &mut App, agent_idx: usize) {
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

fn relay_field_count(app: &App) -> usize {
    match selected_agent_name(app) {
        Some("opencode") => 6,
        _ => 3,
    }
}

fn exit_relay(app: &mut App, host: RelayHost) {
    app.relay_editing = false;
    app.relay_edit_buffer.clear();
    app.clear_relay_popup_state();
    match host {
        RelayHost::Standalone => app.mode = Mode::Settings,
        RelayHost::Settings => app.leave_settings_detail(),
    }
}

fn selected_agent_name(app: &App) -> Option<&str> {
    app.config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.name.as_str())
}

fn uniquify_provider_key(
    agent: &crate::theme::AgentConfig,
    raw: &str,
    skip_idx: Option<usize>,
) -> String {
    let base = normalize_provider_key(raw);
    let mut candidate = base.clone();
    let mut suffix = 2usize;
    loop {
        let conflict = agent.providers.iter().enumerate().any(|(idx, provider)| {
            if Some(idx) == skip_idx {
                return false;
            }
            provider.opencode_provider_key() == candidate
        });
        if !conflict {
            return candidate;
        }
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
}

fn unique_model_id(provider: &ProviderConfig, raw: &str, skip_idx: Option<usize>) -> String {
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

fn model_picker_options(app: &App) -> Vec<(String, String)> {
    let mut options = app
        .config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.opencode_model_options())
        .unwrap_or_default();

    if app.relay_popup_mode == RelayPopupMode::OpenCodeSmallModel {
        options.insert(0, (String::new(), "(none)".to_string()));
    }

    options
}

fn selected_model_picker_index(app: &App, include_none: bool) -> usize {
    let current = app
        .config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| {
            if include_none {
                agent.small_model.as_str()
            } else {
                agent.default_model.as_str()
            }
        })
        .unwrap_or_default();

    model_picker_options(app)
        .iter()
        .position(|(value, _)| value == current)
        .unwrap_or(0)
}

fn export_selected_codex_provider(app: &mut App) {
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

fn import_selected_codex_provider(app: &mut App) {
    match relay::read_codex_relay_import() {
        Ok((providers, active_provider, path)) => {
            let agent_idx = app.relay_selected_agent;
            if let Some(agent) = app.config.agents.get_mut(agent_idx) {
                agent.providers = providers;
                agent.active_provider = active_provider;
                agent.base_url = None;
                agent.api_key = None;
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
