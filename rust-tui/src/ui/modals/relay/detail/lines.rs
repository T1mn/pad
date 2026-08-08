mod codex {
    use crate::theme::{AgentConfig, ProviderConfig, Theme};
    use ratatui::{
        style::Style,
        text::{Line, Span},
    };

    pub(super) fn append_codex_source_line(
        detail_lines: &mut Vec<Line<'static>>,
        agent: &AgentConfig,
        provider: &ProviderConfig,
        theme: &Theme,
    ) {
        if agent.name != "codex" {
            return;
        }
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(Span::styled(
            format!(
                "auth.json: {}  ·  pad.config.toml: {}",
                super::super::super::layout::yes_no(provider.codex_auth_token().is_some()),
                super::super::super::layout::yes_no(!provider.base_url.trim().is_empty())
            ),
            Style::default().fg(theme.comment),
        )));
    }
}
mod default {
    use crate::i18n::Locale;
    use crate::theme::{ProviderConfig, Theme};
    use ratatui::{
        style::Style,
        text::{Line, Span},
    };

    pub(super) fn default_detail_lines(
        provider: &ProviderConfig,
        theme: &Theme,
        locale: Locale,
        make_val: &impl Fn(usize, &str) -> String,
        field_style: &impl Fn(usize) -> Style,
        masked_api_key: String,
    ) -> Vec<Line<'static>> {
        vec![
            super::detail_line(theme, locale, "relay.label"),
            Line::from(Span::styled(make_val(0, &provider.label), field_style(0))),
            Line::from(""),
            super::detail_line(theme, locale, "relay.base_url"),
            Line::from(Span::styled(
                make_val(1, &provider.base_url),
                field_style(1),
            )),
            Line::from(""),
            super::detail_line(theme, locale, "relay.api_key"),
            Line::from(Span::styled(masked_api_key, field_style(2))),
        ]
    }
}
mod edit;
mod secret {
    use super::super::super::super::common::mask_secret_prefix;
    use crate::app::App;
    use crate::theme::{AgentConfig, ProviderConfig};

    pub(super) fn masked_api_key(
        app: &App,
        agent: &AgentConfig,
        provider: &ProviderConfig,
    ) -> String {
        if app.relay_editing && app.relay_edit_field == 2 {
            format!("{}|", app.relay_edit_buffer)
        } else if agent.name == "codex" {
            mask_secret_prefix(
                provider.codex_auth_token().as_deref().unwrap_or_default(),
                10,
            )
        } else {
            mask_secret_prefix(&provider.api_key, 12)
        }
    }
}

use super::opencode::{opencode_detail_lines, OpencodeDetailContext};
use super::test_status::{append_provider_test_lines, ProviderTestStatus};
use crate::app::App;
use crate::i18n::Locale;
use crate::theme::{AgentConfig, ProviderConfig, Theme};
pub(in crate::ui::modals::relay::detail) use edit::RelayEditState;
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
        opencode_detail_lines(OpencodeDetailContext {
            agent,
            provider,
            theme,
            locale,
            make_val: &make_val,
            field_style: &field_style,
            masked_api_key,
            edit: &edit,
        })
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

    if agent.name == "claude" {
        detail_lines.push(Line::from(""));
        detail_lines.push(detail_line(theme, locale, "relay.disable_thinking"));
        detail_lines.push(Line::from(Span::styled(
            if provider.disable_thinking {
                "true"
            } else {
                "false"
            },
            field_style(3),
        )));
    }

    codex::append_codex_source_line(&mut detail_lines, agent, provider, theme);

    append_provider_test_lines(
        &mut detail_lines,
        ProviderTestStatus {
            in_progress: app.provider_test_in_progress,
            status: provider.test_status,
            http_status: provider.test_http_status,
            latency_ms: provider.test_latency_ms,
            result: provider.test_result.as_deref(),
            theme,
            locale,
        },
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
