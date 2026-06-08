mod codex;
mod default;
mod edit;
mod secret;

use super::opencode::opencode_detail_lines;
use super::test_status::append_provider_test_lines;
use crate::app::App;
use crate::i18n::Locale;
use crate::theme::{AgentConfig, ProviderConfig, Theme};
use edit::RelayEditState;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use secret::masked_api_key;

pub(super) fn relay_detail_lines(
    app: &App,
    agent: &AgentConfig,
    provider: &ProviderConfig,
    theme: &Theme,
    locale: Locale,
) -> Vec<Line<'static>> {
    let edit = RelayEditState::from_app(app, theme);
    let make_val = |idx: usize, value: &str| edit.value(idx, value);
    let field_style = |idx: usize| edit.field_style(idx);
    let masked_api_key = masked_api_key(app, agent, provider);

    let mut detail_lines = if agent.name == "opencode" {
        opencode_detail_lines(
            agent,
            provider,
            theme,
            locale,
            &make_val,
            &field_style,
            masked_api_key,
            edit.editing,
            edit.field,
            edit.buffer,
        )
    } else {
        default::default_detail_lines(
            provider,
            theme,
            locale,
            &make_val,
            &field_style,
            masked_api_key,
        )
    };

    codex::append_codex_source_line(&mut detail_lines, agent, provider, theme);

    append_provider_test_lines(
        &mut detail_lines,
        app.provider_test_in_progress,
        provider.test_status,
        provider.test_http_status,
        provider.test_latency_ms,
        provider.test_result.as_deref(),
        theme,
        locale,
    );

    detail_lines
}

pub(in crate::ui::modals::relay::detail) fn detail_line(
    theme: &Theme,
    locale: Locale,
    key: &'static str,
) -> Line<'static> {
    Line::from(Span::styled(
        crate::i18n::t(locale, key),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::DIM),
    ))
}
