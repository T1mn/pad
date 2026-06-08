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
