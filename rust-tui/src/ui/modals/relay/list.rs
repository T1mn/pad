use super::super::common::truncate_modal_line;
use super::layout::relay_provider_footer_text;
use crate::app::App;
use crate::theme::ProviderConfig;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::{layout::Rect, Frame};

pub(super) fn draw_relay_settings_content(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let (title, items, selected, footer_text) = match app.relay_view {
        crate::app::state::RelayView::AgentList => {
            let items: Vec<SelectionItem> = app
                .config
                .agents
                .iter()
                .map(|agent| {
                    let active_label = agent
                        .active()
                        .map(|provider| provider.label.clone())
                        .unwrap_or_else(|| {
                            if agent.name == "opencode" && !agent.default_model.is_empty() {
                                agent.default_model.clone()
                            } else {
                                "none".to_string()
                            }
                        });
                    let subtitle = if agent.name == "opencode" {
                        let model = if agent.default_model.is_empty() {
                            "none".to_string()
                        } else {
                            truncate_modal_line(&agent.default_model, 24)
                        };
                        let small = if agent.small_model.is_empty() {
                            "none".to_string()
                        } else {
                            truncate_modal_line(&agent.small_model, 20)
                        };
                        format!(
                            "model: {}  ·  small: {}  ·  {} providers",
                            model,
                            small,
                            agent.providers.len()
                        )
                    } else {
                        format!(
                            "active: {}  ·  {} providers",
                            active_label,
                            agent.providers.len()
                        )
                    };
                    SelectionItem {
                        title: agent.name.clone(),
                        subtitle: Some(subtitle),
                        keyword: Some(format!("{} {}", agent.name, active_label)),
                        detail: None,
                        disabled: false,
                    }
                })
                .collect();
            (
                crate::i18n::t(locale, "relay.title").to_string(),
                items,
                app.relay_selected_agent,
                crate::i18n::t(locale, "relay.footer_agent"),
            )
        }
        crate::app::state::RelayView::ProviderList | crate::app::state::RelayView::DetailPane => {
            let items: Vec<SelectionItem> = if let Some(agent) = selected_agent {
                if agent.providers.is_empty() {
                    vec![SelectionItem {
                        title: crate::i18n::t(locale, "relay.empty").to_string(),
                        subtitle: None,
                        keyword: None,
                        detail: None,
                        disabled: true,
                    }]
                } else {
                    agent
                        .providers
                        .iter()
                        .enumerate()
                        .map(|(idx, provider)| {
                            let is_active = agent.active_provider == Some(idx);
                            SelectionItem {
                                title: provider_list_title(
                                    agent.name.as_str(),
                                    provider,
                                    is_active,
                                ),
                                subtitle: Some(provider_list_subtitle(
                                    agent.name.as_str(),
                                    provider,
                                )),
                                keyword: Some(format!(
                                    "{} {} {}",
                                    provider.label,
                                    provider.base_url,
                                    provider.test_result.clone().unwrap_or_default()
                                )),
                                detail: None,
                                disabled: false,
                            }
                        })
                        .collect()
                }
            } else {
                vec![]
            };
            (
                format!(
                    "{} / {}",
                    crate::i18n::t(locale, "relay.providers_label"),
                    selected_agent
                        .map(|agent| agent.name.as_str())
                        .unwrap_or("?")
                ),
                items,
                app.relay_selected_provider,
                relay_provider_footer_text(app, locale),
            )
        }
    };

    let mut state = SelectionState {
        selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(f, area, theme, &title, &items, &state, Some(footer_text));
}

fn provider_list_title(agent_name: &str, provider: &ProviderConfig, is_active: bool) -> String {
    let marker = if is_active { "✓ " } else { "" };
    if agent_name == "opencode" {
        format!(
            "{}{} [{}]",
            marker,
            provider.label,
            provider.opencode_provider_key()
        )
    } else {
        format!("{}{}", marker, provider.label)
    }
}

fn provider_list_subtitle(agent_name: &str, provider: &ProviderConfig) -> String {
    let mut parts = Vec::new();
    if agent_name == "opencode" {
        parts.push(format!("{} models", provider.models.len()));
        parts.push(truncate_modal_line(provider.opencode_npm_package(), 24));
    }
    if !provider.base_url.trim().is_empty() {
        parts.push(truncate_modal_line(&provider.base_url, 28));
    }
    if agent_name == "codex" {
        parts.push(format!(
            "auth {}",
            super::layout::yes_no(provider.codex_auth_token().is_some())
        ));
    }
    if let Some(ok) = provider.test_status {
        parts.push(if ok {
            "probe ok".to_string()
        } else {
            "probe failed".to_string()
        });
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("  |  ")
    }
}
