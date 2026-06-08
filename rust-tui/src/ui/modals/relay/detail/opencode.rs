use super::lines::detail_line;
use crate::i18n::Locale;
use crate::theme::{AgentConfig, ProviderConfig, Theme};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

#[allow(clippy::too_many_arguments)]
pub(super) fn opencode_detail_lines(
    agent: &AgentConfig,
    provider: &ProviderConfig,
    theme: &Theme,
    locale: Locale,
    make_val: &impl Fn(usize, &str) -> String,
    field_style: &impl Fn(usize) -> Style,
    masked_api_key: String,
    editing: bool,
    field: usize,
    edit_buffer: &str,
) -> Vec<Line<'static>> {
    let models_summary = opencode_models_summary(provider);
    vec![
        detail_line(theme, locale, "relay.label"),
        Line::from(Span::styled(make_val(0, &provider.label), field_style(0))),
        Line::from(""),
        detail_line(theme, locale, "relay.provider_key"),
        Line::from(Span::styled(
            make_val(1, provider.opencode_provider_key()),
            field_style(1),
        )),
        Line::from(""),
        detail_line(theme, locale, "relay.npm_package"),
        Line::from(Span::styled(
            make_val(2, provider.opencode_npm_package()),
            field_style(2),
        )),
        Line::from(""),
        detail_line(theme, locale, "relay.base_url"),
        Line::from(Span::styled(
            make_val(3, &provider.base_url),
            field_style(3),
        )),
        Line::from(""),
        detail_line(theme, locale, "relay.api_key"),
        Line::from(Span::styled(
            if editing && field == 4 {
                format!("{}|", edit_buffer)
            } else {
                masked_api_key
            },
            field_style(4),
        )),
        Line::from(""),
        detail_line(theme, locale, "relay.models"),
        Line::from(Span::styled(models_summary, field_style(5))),
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "model: {}  ·  small: {}",
                dash_if_empty(&agent.default_model),
                dash_if_empty(&agent.small_model)
            ),
            Style::default().fg(theme.comment),
        )),
    ]
}

fn dash_if_empty(value: &str) -> String {
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        super::super::super::common::truncate_modal_line(value, 24)
    }
}

fn opencode_models_summary(provider: &ProviderConfig) -> String {
    if provider.models.is_empty() {
        return "-".to_string();
    }
    let preview = provider
        .models
        .iter()
        .take(2)
        .map(|model| {
            if model.name.trim().is_empty() {
                model.id.clone()
            } else {
                format!("{} ({})", model.name, model.id)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    if provider.models.len() > 2 {
        format!(
            "{}  ·  +{} more",
            super::super::super::common::truncate_modal_line(&preview, 40),
            provider.models.len() - 2
        )
    } else {
        super::super::super::common::truncate_modal_line(&preview, 48)
    }
}
