use super::lines::{detail_line, RelayEditState};
use crate::i18n::Locale;
use crate::theme::{AgentConfig, ProviderConfig, Theme};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(super) struct OpencodeDetailContext<'a, MakeVal, FieldStyle>
where
    MakeVal: Fn(usize, &str) -> String,
    FieldStyle: Fn(usize) -> Style,
{
    pub(super) agent: &'a AgentConfig,
    pub(super) provider: &'a ProviderConfig,
    pub(super) theme: &'a Theme,
    pub(super) locale: Locale,
    pub(super) make_val: &'a MakeVal,
    pub(super) field_style: &'a FieldStyle,
    pub(super) masked_api_key: String,
    pub(super) edit: &'a RelayEditState<'a>,
}

pub(super) fn opencode_detail_lines<MakeVal, FieldStyle>(
    ctx: OpencodeDetailContext<'_, MakeVal, FieldStyle>,
) -> Vec<Line<'static>>
where
    MakeVal: Fn(usize, &str) -> String,
    FieldStyle: Fn(usize) -> Style,
{
    let models_summary = opencode_models_summary(ctx.provider);
    vec![
        detail_line(ctx.theme, ctx.locale, "relay.label"),
        Line::from(Span::styled(
            (ctx.make_val)(0, &ctx.provider.label),
            (ctx.field_style)(0),
        )),
        Line::from(""),
        detail_line(ctx.theme, ctx.locale, "relay.provider_key"),
        Line::from(Span::styled(
            (ctx.make_val)(1, ctx.provider.opencode_provider_key()),
            (ctx.field_style)(1),
        )),
        Line::from(""),
        detail_line(ctx.theme, ctx.locale, "relay.npm_package"),
        Line::from(Span::styled(
            (ctx.make_val)(2, ctx.provider.opencode_npm_package()),
            (ctx.field_style)(2),
        )),
        Line::from(""),
        detail_line(ctx.theme, ctx.locale, "relay.base_url"),
        Line::from(Span::styled(
            (ctx.make_val)(3, &ctx.provider.base_url),
            (ctx.field_style)(3),
        )),
        Line::from(""),
        detail_line(ctx.theme, ctx.locale, "relay.api_key"),
        Line::from(Span::styled(
            if ctx.edit.editing && ctx.edit.field == 4 {
                format!("{}|", ctx.edit.buffer)
            } else {
                ctx.masked_api_key
            },
            (ctx.field_style)(4),
        )),
        Line::from(""),
        detail_line(ctx.theme, ctx.locale, "relay.models"),
        Line::from(Span::styled(models_summary, (ctx.field_style)(5))),
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "model: {}  ·  small: {}",
                dash_if_empty(&ctx.agent.default_model),
                dash_if_empty(&ctx.agent.small_model)
            ),
            Style::default().fg(ctx.theme.comment),
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
    let mut preview = String::new();
    for model in provider.models.iter().take(2) {
        if !preview.is_empty() {
            preview.push_str(", ");
        }
        push_model_summary(&mut preview, model);
    }
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

fn push_model_summary(preview: &mut String, model: &crate::theme::OpenCodeModelConfig) {
    if model.name.trim().is_empty() {
        preview.push_str(&model.id);
    } else {
        preview.push_str(&model.name);
        preview.push_str(" (");
        preview.push_str(&model.id);
        preview.push(')');
    }
}

#[cfg(test)]
#[path = "opencode_tests.rs"]
mod tests;
